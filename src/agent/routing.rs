use super::conversation::WorkflowMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryIntentClass {
    ProductTruth,
    RuntimeDiagnosis,
    RepoArchitecture,
    Toolchain,
    Capability,
    Implementation,
    Research,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectAnswerKind {
    About,
    LanguageCapability,
    UnsafeWorkflowPressure,
    SessionMemory,
    RecoveryRecipes,
    McpLifecycle,
    AuthorizationPolicy,
    ToolClasses,
    ToolRegistryOwnership,
    SessionResetSemantics,
    ProductSurface,
    ReasoningSplit,
    Identity,
    WorkflowModes,
    GemmaNative,
    GemmaNativeSettings,
    VerifyProfiles,
    Toolchain,
    HostInspection,
    ArchitectSessionResetPlan,
    Help,
}

#[derive(Clone, Copy, Debug)]
pub struct QueryIntent {
    pub primary_class: QueryIntentClass,
    pub direct_answer: Option<DirectAnswerKind>,
    pub grounded_trace_mode: bool,
    pub capability_mode: bool,
    pub capability_needs_repo: bool,
    pub toolchain_mode: bool,
    pub host_inspection_mode: bool,
    pub maintainer_workflow_mode: bool,
    pub workspace_workflow_mode: bool,
    pub architecture_overview_mode: bool,
    pub sovereign_mode: bool,
    pub surgical_filesystem_mode: bool,
    pub scaffold_mode: bool,
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

const CODE_KEYWORDS: &[&str] = &[
    ".rs",
    ".js",
    ".ts",
    ".py",
    ".go",
    ".c",
    ".cpp",
    ".h",
    ".hpp",
    ".css",
    ".html",
    ".json",
    ".toml",
    ".yaml",
    ".yml",
    ".md",
    ".sh",
    ".ps1",
    ".sql",
    "rust",
    "python",
    "javascript",
    "typescript",
    "golang",
    "react",
    "svelte",
    "vue",
    "nextjs",
    "node",
    "npm",
    "cargo",
    "pip",
    "logic",
    "refactor",
    "implementation",
    "styles",
    "script",
];

static CODE_KW_AC: std::sync::OnceLock<aho_corasick::AhoCorasick> = std::sync::OnceLock::new();

fn code_kw_ac() -> &'static aho_corasick::AhoCorasick {
    CODE_KW_AC
        .get_or_init(|| aho_corasick::AhoCorasick::new(CODE_KEYWORDS).expect("valid patterns"))
}

fn mentions_reset_commands(lower: &str) -> bool {
    contains_all(lower, &["/clear", "/new", "/forget"])
}

fn mentions_stable_product_surface(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "stable product-surface question",
            "stable product surface question",
            "stable product-surface questions",
            "stable product surface questions",
        ],
    )
}

fn mentions_product_truth_routing(lower: &str) -> bool {
    let asks_decision_policy = contains_any(
        lower,
        &[
            "how hematite decides",
            "how does hematite decide",
            "decides whether",
            "decide whether",
        ],
    );
    let asks_direct_vs_inspect_split = contains_any(
        lower,
        &[
            "answered as stable product truth",
            "stable product truth",
            "stable product behavior",
            "answer directly",
            "direct answer",
            "inspect the repository",
            "inspect repository",
            "repository implementation",
            "repo implementation",
        ],
    );
    asks_decision_policy && asks_direct_vs_inspect_split
}

fn mentions_broad_system_walkthrough(lower: &str) -> bool {
    let asks_walkthrough = contains_any(
        lower,
        &[
            "walk me through",
            "walk through",
            "how hematite is wired",
            "understand how hematite is wired",
            "major runtime pieces",
            "normal message moves",
            "moves from the tui to the model and back",
        ],
    );
    let asks_multiple_runtime_areas = contains_any(
        lower,
        &[
            "session recovery",
            "tool policy",
            "mcp state",
            "mcp policy",
            "files own the major runtime pieces",
            "which files own",
            "where session recovery",
            "where tool policy",
            "where mcp state",
        ],
    );
    asks_walkthrough && asks_multiple_runtime_areas
}

fn mentions_research_query(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "search for",
            "lookup",
            "look up",
            "google",
            "find info",
            "find information",
            "what are the latest",
            "who is",
            "who are",
            "who was",
            "what is",
            "what was",
            "who's",
            "current version of",
            "history of",
            "what happened with",
            "tell me about",
            "tell me about the new",
        ],
    )
}

fn mentions_codebase_keywords(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "this repo",
            "the repo",
            "this project",
            "the project",
            "in the code",
            "in my code",
            "this codebase",
            "the codebase",
            "function",
            "module",
            "file",
            "struct",
            "enum",
            "impl",
            "trait",
            "crate",
            "logic",
            "implementation",
            "wiring",
            "handles ",
            "defined",
            "located",
        ],
    )
}

fn mentions_capability_question(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "what is your purpose",
            "what's your purpose",
            "what are you for",
            "what is your job",
            "what's your job",
            "what can you do",
            "what are you capable",
            "can you make projects",
            "can you build projects",
            "do you know other coding languages",
            "other coding languages",
            "what languages",
            "can you use the internet",
            "internet research capabilities",
            "what tools do you have",
        ],
    )
}

fn mentions_creator_question(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "who is ocean bennett",
            "who's ocean bennett",
            "tell me about ocean bennett",
            "who created you",
            "who built you",
            "who made you",
            "who developed you",
            "who engineered you",
            "who engineered your architecture",
            "who created hematite",
            "who built hematite",
            "who developed hematite",
            "who engineered hematite",
            "who maintains hematite",
            "who authored hematite",
            "who is the author",
            "who wrote this",
            "who made this app",
        ],
    )
}

fn capability_question_requires_repo_inspection(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "this repo",
            "this repository",
            "codebase",
            "which files",
            "implementation",
            "in this project",
        ],
    )
}

/// Returns true for conversational, advisory, or declarative turns that should not
/// trigger a blanket inspect_host(summary) call. Covers:
///   - Advisory/opinion questions: "would more ram be nice?", "should I upgrade?"
///   - Opinion assertions: "i think the gpu is fine"
///   - Hypotheticals: "what if I had more ram", "if i upgraded the gpu"
///   - Conversational acknowledgments: "makes sense", "so the cpu is fine", "ok so"
///   - Positive/negative statements that aren't asking for new data
///
/// Does NOT block specific diagnostic routes — those fire before this catch-all guard.
fn is_conversational_advisory(lower: &str) -> bool {
    // ── Advisory openers — seeking opinion or recommendation, not data ──────────
    let starts_advisory = lower.starts_with("would ")
        || lower.starts_with("could ")
        || lower.starts_with("should ")
        || lower.starts_with("is that ")
        || lower.starts_with("was that ")
        || lower.starts_with("do you think")
        || lower.starts_with("what do you think")
        || lower.starts_with("does that ")
        || lower.starts_with("is it worth")
        || lower.starts_with("would it ");

    // ── Opinion / belief assertions — not requesting fresh data ─────────────────
    let opinion_opener = (lower.starts_with("i think ")
        || lower.starts_with("i believe ")
        || lower.starts_with("i know ")
        || lower.starts_with("i guess ")
        || lower.starts_with("i see,")
        || lower.starts_with("i see ")
        || lower.starts_with("i feel like"))
        && !lower.trim_end().ends_with('?');

    // ── Hypotheticals — not asking about current machine state ──────────────────
    let hypothetical = lower.starts_with("what if ")
        || lower.starts_with("if i ")
        || lower.starts_with("if i'd ")
        || lower.starts_with("say i ")
        || lower.starts_with("suppose ");

    // ── Conversational acknowledgments / pivots without a follow-up question ────
    let no_question = !lower.trim_end().ends_with('?');
    let no_imperative = !lower.contains("what is ")
        && !lower.contains("what are ")
        && !lower.contains("how do ")
        && !lower.contains("how much ")
        && !lower.contains("how many ")
        && !lower.contains("show me")
        && !lower.contains("tell me")
        && !lower.contains("check ");
    let acknowledgment = (lower.starts_with("makes sense")
        || lower.starts_with("that makes sense")
        || lower.starts_with("ok so ")
        || lower.starts_with("right so ")
        || lower.starts_with("so the ")
        || lower.starts_with("so it ")
        || lower.starts_with("so my ")
        || lower.starts_with("ah ")
        || lower.starts_with("got it")
        || lower.starts_with("ok, ")
        || lower.starts_with("everything "))
        && no_question
        && no_imperative;

    // ── Confirmation-seeking tail — "right?", "correct?" ────────────────────────
    let ends_confirmation = lower.trim_end_matches(['?', ' ']).ends_with("right")
        || lower.trim_end_matches(['?', ' ']).ends_with("correct")
        || lower.ends_with("right?")
        || lower.ends_with("yeah?");

    // ── Advisory tail vocabulary ─────────────────────────────────────────────────
    let advisory_tail = lower.contains(" be nice")
        || lower.contains(" be worth")
        || lower.contains(" be helpful")
        || lower.contains(" be useful")
        || lower.contains(" be better")
        || lower.contains(" be good")
        || lower.contains(" help with")
        || lower.contains("offload")
        || lower.contains("upgrade");

    starts_advisory
        || opinion_opener
        || hypothetical
        || acknowledgment
        || ends_confirmation && advisory_tail
}

fn mentions_host_inspection_question(lower: &str) -> bool {
    let host_scope = lower.split_whitespace().any(|w| {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        matches!(
            w,
            "path"
                | "pip"
                | "winget"
                | "choco"
                | "scoop"
                | "network"
                | "adapter"
                | "dns"
                | "gateway"
                | "wifi"
                | "ethernet"
                | "service"
                | "services"
                | "daemon"
                | "process"
                | "processes"
                | "ram"
                | "cpu"
                | "gpu"
                | "vram"
                | "nvidia"
                | "memory"
                | "machine"
                | "computer"
                | "firewall"
                | "vpn"
                | "proxy"
                | "internet"
                | "online"
                | "connectivity"
                | "uptime"
                | "reboot"
                | "silicon"
                | "throttle"
                | "throttled"
                | "throttling"
                | "clocks"
                | "mhz"
                | "health"
                | "report"
                | "bitlocker"
                | "rdp"
                | "vss"
                | "pagefile"
                | "swap"
                | "printer"
                | "audio"
                | "sound"
                | "speaker"
                | "speakers"
                | "microphone"
                | "mic"
                | "bluetooth"
                | "pairing"
                | "headset"
                | "headphones"
                | "camera"
                | "webcam"
                | "msi"
                | "msiexec"
                | "onedrive"
                | "indexer"
                | "ntp"
                | "w32tm"
                | "winrm"
                | "psremoting"
                | "slat"
                | "error"
                | "warning"
                | "event"
                | "log"
                | "throughput"
                | "registry"
                | "share"
                | "mbps"
                | "ad"
                | "sid"
                | "vm"
                | "hyper-v"
                | "hyperv"
                | "dhcp"
                | "lease"
                | "login"
                | "disk"
                | "drive"
                | "backup"
                | "ssd"
                | "hdd"
                | "nvme"
                | "encryption"
        )
    }) || contains_any(
        lower,
        &[
            "package manager",
            "environment doctor",
            "ip address",
            "ipconfig",
            "task manager",
            "developer tools",
            "toolchains",
            "local development",
            "tcp connection",
            "active connection",
            "traceroute",
            "tracert",
            "dns cache",
            "arp table",
            "route table",
            "routing table",
            "default gateway",
            "power plan",
            "windows feature",
            "optional feature",
            "microsoft store",
            "app installer",
            "search index",
            "windows search",
            "monitor resolution",
            "display config",
            "refresh rate",
            "sign in",
            "hard drive",
            "task scheduler",
        ],
    );

    let host_action = lower.split_whitespace().any(|w| {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        matches!(
            w,
            "inspect"
                | "count"
                | "summarize"
                | "analyze"
                | "missing"
                | "ready"
                | "resolve"
                | "troubleshoot"
                | "show"
                | "find"
                | "list"
                | "audit"
                | "test"
                | "check"
                | "currently"
                | "status"
                | "stats"
                | "vitals"
                | "telemetry"
                | "looking"
        )
    }) || contains_any(lower, &["tell me", "how big", "show me"]);

    // Some words are self-sufficient diagnostic state indicators: asking "is my GPU
    // throttled?" implicitly asks to inspect whether throttling is happening.
    let self_sufficient_state =
        lower.contains("throttl") || lower.contains("overheat") || lower.contains("bottleneck");

    host_scope && (host_action || self_sufficient_state)
}

pub fn preferred_host_inspection_topic(user_input: &str) -> Option<&'static str> {
    let lower = user_input.to_lowercase();
    let asks_fix_plan = (lower.contains("fix")
        || lower.contains("repair")
        || lower.contains("resolve")
        || lower.contains("troubleshoot"))
        && (lower.contains("cargo")
            || lower.contains("path")
            || lower.contains("package manager")
            || lower.contains("toolchain")
            || lower.contains("port ")
            || lower.contains("already in use")
            || lower.contains("lm studio")
            || lower.contains("localhost:1234")
            || lower.contains("embedding model")
            || lower.contains("no coding model loaded"));
    let asks_path = lower.contains("path entries")
        || lower.contains("raw path")
        || lower.contains("path issue")
        || lower.contains("missing from path")
        || lower.contains("not in path")
        || lower.contains("path variable")
        || (lower.contains("path") && (lower.contains("show") || lower.contains("what is")));
    let asks_gpo = lower.contains("gpo")
        || lower.contains("group polic")
        || lower.contains("gpresult")
        || lower.contains("applied policy")
        || lower.contains("active policies")
        || lower.contains("what policies")
        || lower.contains("policy objects")
        || lower.contains("policy applied")
        || (lower.contains("policies") && lower.contains("applied"))
        || (lower.contains("policies") && lower.contains("effect"))
        || (lower.contains("policy") && lower.contains("in effect"));
    let asks_certificates = lower.contains("cert")
        || lower.contains("ssl")
        || lower.contains("client cert")
        || lower.contains("expiring cert")
        || lower.contains("tls certificate")
        || lower.contains("x509")
        || lower.contains("x.509")
        || lower.contains(".pfx")
        || lower.contains(".p12")
        || lower.contains(".pem")
        || lower.contains("pkcs")
        || lower.contains("trust store")
        || lower.contains("certificate store")
        || lower.contains("certificate expir")
        || lower.contains("untrusted cert")
        || (lower.contains("tls")
            && (lower.contains("check")
                || lower.contains("inspect")
                || lower.contains("status")
                || lower.contains("valid")));
    let asks_integrity = lower.contains("integrity")
        || lower.contains("sfc")
        || lower.contains("dism")
        || lower.contains("corrupt")
        || lower.contains("os health")
        || lower.contains("system file")
        || (lower.contains("windows") && lower.contains("damaged"))
        || (lower.contains("check") && lower.contains("system") && lower.contains("file"));
    let asks_user_accounts = (lower.contains("user account")
        && !lower.contains("user account control"))
        || lower.contains("local user")
        || lower.contains("local group")
        || lower.contains("get-localuser")
        || lower.contains("get-localgroup")
        || lower.contains("get-localgroupmember")
        || lower.contains("who am i")
        || lower.contains("logged in as")
        || lower.contains("logged in user")
        || lower.contains("logged on user")
        || lower.contains("admin group")
        || lower.contains("administrators group")
        || lower.contains("local admin")
        || lower.contains("who has admin")
        || lower.contains("running as admin")
        || lower.contains("is this elevated")
        || lower.contains("net user")
        || lower.contains("net localgroup")
        || lower.contains("who has admin rights")
        || lower.contains("list all users")
        || lower.contains("list users")
        || lower.contains("what accounts")
        || (lower.contains("accounts") && lower.contains("admin"));
    let asks_ad_user = lower.contains("ad user")
        || lower.contains("domain user")
        || (lower.contains("user") && (lower.contains("sid") || lower.contains("membership")));
    let asks_mdm = lower.contains("mdm")
        || lower.contains("intune")
        || lower.contains("autopilot")
        || lower.contains("device enrollment")
        || lower.contains("enrolled in")
        || lower.contains("mdm enrollment")
        || lower.contains("device management")
        || lower.contains("managed device")
        || lower.contains("azure ad join")
        || lower.contains("aad join")
        || (lower.contains("enrolled") && lower.contains("device"))
        || (lower.contains("enroll") && lower.contains("device"))
        || (lower.contains("microsoft") && lower.contains("endpoint"));
    let asks_hyperv = lower.contains("hyper-v")
        || lower.contains("hyperv")
        || lower.contains("hyper v")
        || lower.contains("list vm")
        || lower.contains("list vms")
        || lower.contains("running vms")
        || lower.contains("virtual machines")
        || lower.contains("virtual machine")
        || (lower.contains("vm")
            && !lower.contains("nvme")
            && (lower.contains("running")
                || lower.contains("status")
                || lower.contains("health")
                || lower.contains("checkpoint")
                || lower.contains("snapshot")
                || lower.contains("switch")
                || lower.contains("memory")
                || lower.contains("ram")))
        || lower.contains("vmms")
        || lower.contains("vmmem");
    let asks_storage_spaces = lower.contains("storage space")
        || lower.contains("storage pool")
        || lower.contains("storage pools")
        || lower.contains("virtual disk")
        || lower.contains("virtual disks")
        || lower.contains("windows raid")
        || lower.contains("disk pool")
        || lower.contains("resiliency")
        || (lower.contains("storage") && lower.contains("pool"))
        || (lower.contains("mdadm")
            || lower.contains("software raid")
            || lower.contains("md array"));
    let asks_defender_quarantine = lower.contains("quarantine")
        || lower.contains("threat history")
        || lower.contains("malware history")
        || lower.contains("defender history")
        || lower.contains("detected threat")
        || lower.contains("detected virus")
        || lower.contains("defender found")
        || lower.contains("defender detected")
        || lower.contains("defender find")
        || lower.contains("virus found")
        || lower.contains("threats found")
        || lower.contains("threat detected")
        || (lower.contains("defender")
            && (lower.contains("malware") || lower.contains("virus") || lower.contains("threat")))
        || (lower.contains("defender")
            && lower.contains("scan")
            && (lower.contains("result") || lower.contains("history") || lower.contains("found")));
    let asks_event_query = lower.contains("event id")
        || lower.contains("event log query")
        || lower.contains("event_id")
        || lower.contains("eventid")
        || lower.contains("search event")
        || lower.contains("query event")
        || lower.contains("find event")
        || lower.contains("filter event")
        || (lower.contains("event") && lower.contains("4625"))
        || (lower.contains("event") && lower.contains("7034"))
        || (lower.contains("event") && lower.contains("7031"))
        || (lower.contains("event") && lower.contains("4648"))
        || (lower.contains("event") && lower.contains("41"))
        || (lower.contains("event")
            && (lower.contains("last hour")
                || lower.contains("last 24")
                || lower.contains("past hour")
                || lower.contains("today")))
        || ((lower.contains("event log")
            || lower.contains("system log")
            || lower.contains("application log")
            || lower.contains("security log"))
            && (lower.contains("last ")
                || lower.contains("past ")
                || lower.contains("today")
                || lower.contains("hour")
                || lower.contains("hours"))
            && (lower.contains("error")
                || lower.contains("errors")
                || lower.contains("warning")
                || lower.contains("warnings")
                || lower.contains("critical")))
        || lower.contains("failed logon event")
        || lower.contains("failed login event")
        || lower.contains("application error event")
        || lower.contains("crash event")
        || lower.contains("service crash event");
    let asks_ip_config =
        lower.contains("ipconfig") && (lower.contains("all") || lower.contains("detailed"));
    let asks_domain = lower.contains("domain")
        || lower.contains("active directory")
        || lower.contains("ad join")
        || lower.contains("workgroup");
    let asks_device_health = lower.contains("device health")
        || lower.contains("hardware error")
        || lower.contains("malfunctioning")
        || lower.contains("yellow bang")
        || lower.contains("hardware failing")
        || lower.contains("device manager")
        || lower.contains("unknown device")
        || lower.contains("code 43")
        || lower.contains("code 10")
        || lower.contains("code 28")
        || lower.contains("pnp device")
        || lower.contains("exclamation mark in device")
        || (lower.contains("device") && lower.contains("error code"))
        || (lower.contains("device") && lower.contains("not recognized"))
        || (lower.contains("device") && lower.contains("broken"))
        || (lower.contains("device") && lower.contains("not working"))
        || (lower.contains("device") && lower.contains("stopped working"))
        || (lower.contains("hardware") && lower.contains("broken"));
    let asks_drivers =
        lower.contains("driver") || lower.contains("kmod") || lower.contains("kernel module");
    let asks_audio = lower.contains("no sound")
        || lower.contains("audio service")
        || lower.contains("windows audio")
        || lower.contains("speaker")
        || lower.contains("speakers")
        || lower.contains("microphone")
        || lower.contains(" mic ")
        || lower.starts_with("mic ")
        || lower.contains("mic not")
        || lower.contains("headset")
        || lower.contains("headphones")
        || lower.contains("playback device")
        || lower.contains("recording device")
        || lower.contains("audio endpoint")
        || lower.contains("audioendpointbuilder")
        || lower.contains("can't hear")
        || lower.contains("cannot hear")
        || lower.contains("cant hear")
        || lower.contains("no audio")
        || ((lower.contains("audio") || lower.contains("sound"))
            && (lower.contains("device")
                || lower.contains("driver")
                || lower.contains("service")
                || lower.contains("working")
                || lower.contains("broken")
                || lower.contains("input")
                || lower.contains("output")
                || lower.contains("crackling")
                || lower.contains("mute")
                || lower.contains("muted")
                || lower.contains("volume")
                || lower.contains("speaker")
                || lower.contains("microphone")))
            && !lower.contains("audio file")
            && !lower.contains("voice engine");
    let asks_bluetooth = lower.contains("bluetooth")
        || lower.contains("pairing")
        || lower.contains("paired device")
        || lower.contains("paired devices")
        || lower.contains("bthserv")
        || lower.contains("bthavctpsvc")
        || lower.contains("btagservice")
        || lower.contains("bluetoothuserservice")
        || lower.contains("wireless headset")
        || lower.contains("wireless earbuds")
        || ((lower.contains("headset") || lower.contains("headphones"))
            && (lower.contains("disconnect")
                || lower.contains("pair")
                || lower.contains("reconnect")
                || lower.contains("bluetooth")))
        || ((lower.contains("won't") || lower.contains("cannot") || lower.contains("can't"))
            && (lower.contains("pair") || lower.contains("connect"))
            && lower.contains("bluetooth"));
    let asks_camera = lower.contains("camera")
        || lower.contains("webcam")
        || lower.contains("web cam")
        || (lower.contains("app") && lower.contains("can't see") && lower.contains("camera"))
        || (lower.contains("camera") && lower.contains("permission"))
        || (lower.contains("camera") && lower.contains("privacy"))
        || (lower.contains("camera") && lower.contains("not working"))
        || (lower.contains("camera") && lower.contains("missing"))
        || lower.contains("camera_privacy");
    let asks_sign_in = lower.contains("windows hello")
        || (lower.contains("hello") && lower.contains("not working"))
        || (lower.contains("pin")
            && (lower.contains("broken")
                || lower.contains("not working")
                || lower.contains("forgot")))
        || (lower.contains("can't sign in")
            || lower.contains("cannot sign in")
            || lower.contains("cant sign in"))
        || (lower.contains("sign") && lower.contains("in") && lower.contains("issue"))
        || lower.contains("logon failure")
        || lower.contains("credential provider")
        || lower.contains("biometric service")
        || (lower.contains("profile") && lower.contains("corrupt"))
        || lower.contains("wbiosrvc")
        || lower.contains("login screen stuck")
        || lower.contains("stuck on login")
        || lower.contains("stuck at login")
        || lower.contains("login loop")
        || lower.contains("sign-in loop")
        || lower.contains("sign in loop")
        || lower.contains("reboot loop on login")
        || (lower.contains("can't log in") && !lower.contains("vpn") && !lower.contains("ssh"))
        || (lower.contains("cannot log in") && !lower.contains("vpn") && !lower.contains("ssh"))
        || lower.contains("can't login")
        || lower.contains("cannot login")
        || lower.contains("cant login")
        || lower.contains("login failed")
        || lower.contains("login failure")
        || (lower.contains("login") && lower.contains("not working"))
        || (lower.contains("login") && lower.contains("problem"))
        || (lower.contains("login") && lower.contains("status"))
        || lower.contains("sign-in status")
        || lower.contains("sign in status");
    let asks_identity_auth = lower.contains("web account manager")
        || lower.contains("token broker")
        || lower.contains("tokenbroker")
        || lower.contains("aad broker")
        || lower.contains("broker plugin")
        || lower.contains("identity broker")
        || lower.contains("microsoft 365 sign-in")
        || lower.contains("microsoft 365 signin")
        || lower.contains("office sign-in")
        || lower.contains("office signin")
        || lower.contains("workplace join")
        || lower.contains("device registration")
        || lower.contains("device registered")
        || lower.contains("entra")
        || lower.contains("azure ad")
        || lower.contains("azuread")
        || lower.contains("azure ad prt")
        || lower.contains("azureadprt")
        || lower.contains("wamdefaultset")
        || lower.contains("single sign-on")
        || lower.contains("organizational account")
        || lower.contains("corporate account")
        || (lower.contains("azure") && lower.contains("registered"))
        || ((lower.contains("outlook")
            || lower.contains("teams")
            || lower.contains("onedrive")
            || lower.contains("office")
            || lower.contains("microsoft 365"))
            && (lower.contains("sign in")
                || lower.contains("signin")
                || lower.contains("signed in")
                || lower.contains("signed out")
                || lower.contains("keeps asking")
                || lower.contains("keep asking")
                || lower.contains("authentication")
                || lower.contains("auth")
                || lower.contains("token")
                || lower.contains("work account")
                || lower.contains("school account")
                || lower.contains("account mismatch")));
    let asks_installer_health = lower.contains("installer health")
        || lower.contains("installer broken")
        || lower.contains("msiexec")
        || lower.contains("msi installer")
        || lower.contains("windows installer")
        || lower.contains("app installer")
        || lower.contains("desktopappinstaller")
        || lower.contains("microsoft store")
        || lower.contains("winget broken")
        || (lower.contains("can't install")
            && (lower.contains("app") || lower.contains("apps") || lower.contains("program")))
        || (lower.contains("cannot install")
            && (lower.contains("app") || lower.contains("apps") || lower.contains("program")))
        || (lower.contains("cant install")
            && (lower.contains("app") || lower.contains("apps") || lower.contains("program")))
        || (lower.contains("unable to install")
            && (lower.contains("app") || lower.contains("program") || lower.contains("software")))
        || ((lower.contains("install") || lower.contains("installer"))
            && (lower.contains("fail")
                || lower.contains("failing")
                || lower.contains("broken")
                || lower.contains("stuck")
                || lower.contains("hanging")
                || lower.contains("error"))
            && !lower.contains("windows update"));
    let asks_onedrive = lower.contains("onedrive")
        || lower.contains("one drive")
        || lower.contains("files on-demand")
        || lower.contains("known folder backup")
        || lower.contains("known folder move")
        || lower.contains("kfm")
        || lower.contains("sharepoint sync")
        || lower.contains("sync root")
        || ((lower.contains("desktop")
            || lower.contains("documents")
            || lower.contains("pictures"))
            && lower.contains("backup")
            && (lower.contains("onedrive") || lower.contains("cloud") || lower.contains("sync")))
        || ((lower.contains("desktop")
            || lower.contains("documents")
            || lower.contains("pictures"))
            && lower.contains("sync")
            && (lower.contains("onedrive")
                || lower.contains("sharepoint")
                || lower.contains("cloud")));
    let asks_browser_health = lower.contains("browser health")
        || lower.contains("webview2")
        || lower.contains("default browser")
        || ((lower.contains("browser")
            || lower.contains("chrome")
            || lower.contains("edge")
            || lower.contains("firefox"))
            && (lower.contains("slow")
                || lower.contains("sluggish")
                || lower.contains("lag")
                || lower.contains("crash")
                || lower.contains("crashing")
                || lower.contains("hang")
                || lower.contains("frozen")
                || lower.contains("freeze")
                || lower.contains("broken")
                || lower.contains("not opening")
                || lower.contains("won't open")
                || lower.contains("cannot open")
                || lower.contains("unresponsive")
                || lower.contains("not starting")
                || lower.contains("not loading")
                || lower.contains("extension")
                || lower.contains("extensions")
                || lower.contains("proxy")
                || lower.contains("policy")))
        || ((lower.contains("links") || lower.contains("link"))
            && (lower.contains("open wrong")
                || lower.contains("opens wrong")
                || lower.contains("wrong browser")
                || lower.contains("wrong app")
                || lower.contains("default browser")))
        || ((lower.contains("website") || lower.contains("websites") || lower.contains("web app"))
            && (lower.contains("browser")
                || lower.contains("chrome")
                || lower.contains("edge")
                || lower.contains("firefox"))
            && (lower.contains("load")
                || lower.contains("broken")
                || lower.contains("slow")
                || lower.contains("proxy")
                || lower.contains("policy")));
    let asks_outlook = lower.contains("outlook")
        || lower.contains("ms outlook")
        || lower.contains("microsoft outlook")
        || (lower.contains("ost") && lower.contains("mail"))
        || (lower.contains("pst") && lower.contains("mail"))
        || (lower.contains("add-in") && lower.contains("mail"))
        || (lower.contains("addin") && lower.contains("outlook"))
        || (lower.contains("email client")
            && (lower.contains("slow")
                || lower.contains("crash")
                || lower.contains("broken")
                || lower.contains("hanging")))
        || (lower.contains("mail profile") && lower.contains("corrupt"));
    let not_nic_teaming = !lower.contains("nic teaming")
        && !lower.contains("nic-teaming")
        && !lower.contains("link aggregation")
        && !lower.contains("lbfo");
    let asks_teams = (lower.contains("teams") && not_nic_teaming)
        || lower.contains("ms teams")
        || lower.contains("microsoft teams")
        || (lower.contains("teams cache") && lower.contains("clear"))
        || (lower.contains("teams")
            && not_nic_teaming
            && lower.contains("sign-in")
            && lower.contains("broken"))
        || (lower.contains("teams")
            && not_nic_teaming
            && lower.contains("device")
            && (lower.contains("audio")
                || lower.contains("video")
                || lower.contains("camera")
                || lower.contains("microphone")));
    let asks_windows_backup = lower.contains("file history")
        || lower.contains("windows backup")
        || lower.contains("wbadmin")
        || lower.contains("system restore")
        || lower.contains("restore point")
        || lower.contains("restore points")
        || lower.contains("backed up")
        || lower.contains("being backed")
        || (lower.contains("backup")
            && !lower.contains("onedrive")
            && !lower.contains("one drive")
            && (lower.contains("backup drive")
                || lower.contains("backup disk")
                || lower.contains("configured")
                || lower.contains("schedule")
                || lower.contains("last backup")
                || lower.contains("backup health")
                || lower.contains("backup status")
                || lower.contains("backup running")
                || lower.contains("broken")
                || lower.contains("failed")
                || lower.contains("running")
                || lower.contains("enabled")
                || lower.contains("working")
                || lower.contains("set up")))
        || (lower.contains("recovery")
            && (lower.contains("backup")
                || lower.contains("restore")
                || lower.contains("posture")))
        || lower.contains("known folder move")
        || lower.contains("known folder backup");
    let asks_search_index = (lower.contains("search")
        && (lower.contains("broken")
            || lower.contains("not working")
            || lower.contains("slow")
            || lower.contains("indexing")
            || lower.contains("index")))
        || lower.contains("wsearch")
        || lower.contains("windows search")
        || lower.contains("search index")
        || lower.contains("indexer")
        || (lower.contains("search") && lower.contains("stuck"))
        || (lower.contains("search") && lower.contains("results") && lower.contains("show"));
    let asks_display_config = lower.contains("monitor")
        || lower.contains("display")
        || lower.contains("resolution")
        || lower.contains("refresh rate")
        || lower.contains("refresh hz")
        || lower.contains("screen config")
        || lower.contains("dpi")
        || lower.contains("scaling")
        || lower.contains("hdmi")
        || lower.contains("displayport")
        || lower.contains("how many screens")
        || lower.contains("multi-monitor")
        || lower.contains("second screen")
        || lower.contains("external display");
    let asks_data_audit = lower.contains("data audit")
        || lower.contains("audit data")
        || lower.contains("csv schema")
        || lower.contains("data schema")
        || lower.contains("inspect file")
        || lower.contains("profile data")
        || lower.contains("data distribution")
        || (lower.contains("audit")
            && (lower.contains("csv")
                || lower.contains("json")
                || lower.contains("file")
                || lower.contains("data")))
        || (lower.contains("schema")
            && (lower.contains("csv") || lower.contains("json") || lower.contains("data")));
    let asks_ntp = lower.contains("ntp")
        || lower.contains("time sync")
        || lower.contains("clock sync")
        || lower.contains("w32tm")
        || lower.contains("clock drift")
        || lower.contains("system clock")
        || lower.contains("time server")
        || lower.contains("time zone")
        || lower.contains("timezone")
        || lower.contains("wrong timezone")
        || (lower.contains("time") && lower.contains("drift"))
        || (lower.contains("clock") && lower.contains("wrong"))
        || (lower.contains("time") && lower.contains("wrong"))
        || (lower.contains("clock") && lower.contains("off"))
        || (lower.contains("time") && lower.contains("off") && lower.contains("sync"))
        || lower.contains("system time")
        || (lower.contains("time") && lower.contains("accurate"))
        || (lower.contains("time") && lower.contains("correct"));
    let asks_cpu_power = lower.contains("turbo boost")
        || lower.contains("cpu frequency")
        || lower.contains("cpu freq")
        || lower.contains("processor frequency")
        || lower.contains("cpu clock")
        || lower.contains("cpu speed")
        || lower.contains("processor speed")
        || lower.contains("cpu stuck")
        || lower.contains("cpu slow")
        || lower.contains("power plan")
        || lower.contains("cpu power")
        || lower.contains("processor state")
        || (lower.contains("cpu") && lower.contains("slow"))
        || (lower.contains("cpu") && lower.contains("underclocking"))
        || (lower.contains("boost") && lower.contains("disabled"))
        || (lower.contains("processor") && lower.contains("slow"))
        || (lower.contains("processor") && lower.contains("running slow"))
        || lower.contains("processor running at");
    let asks_credentials = lower.contains("credential manager")
        || lower.contains("credential store")
        || lower.contains("saved password")
        || lower.contains("stored credential")
        || lower.contains("saved credential")
        || lower.contains("credential vault")
        || lower.contains("cmdkey")
        || (lower.contains("credential") && lower.contains("list"))
        || (lower.contains("password") && lower.contains("vault"))
        || (lower.contains("windows") && lower.contains("credential"))
        || (lower.contains("credential")
            && (lower.contains("clear")
                || lower.contains("cached")
                || lower.contains("view")
                || lower.contains("delete")
                || lower.contains("remove")));
    let asks_tpm = lower.contains("tpm")
        || lower.contains("secure boot")
        || lower.contains("secureboot")
        || lower.contains("trusted platform module")
        || lower.contains("firmware security")
        || lower.contains("uefi security")
        || lower.contains("uefi mode")
        || lower.contains("uefi enabled")
        || lower.contains("uefi settings")
        || lower.contains("legacy bios")
        || lower.contains("uefi bios")
        || (lower.contains("uefi")
            && (lower.contains("boot")
                || lower.contains("secure")
                || lower.contains("status")
                || lower.contains("check")))
        || (lower.contains("bitlocker") && lower.contains("chip"))
        || (lower.contains("windows 11") && lower.contains("tpm"));
    let asks_dhcp = lower.contains("dhcp lease")
        || lower.contains("lease expires")
        || lower.contains("lease obtained")
        || lower.contains("dhcp server")
        || lower.contains("ip lease")
        || lower.contains("lease time")
        || lower.contains("lease renew")
        || lower.contains("renew lease")
        || (lower.contains("dhcp")
            && (lower.contains("detail")
                || lower.contains("info")
                || lower.contains("check")
                || lower.contains("show")))
        || (lower.contains("ip") && lower.contains("lease"));
    let asks_mtu = lower.contains("mtu")
        || lower.contains("path mtu")
        || lower.contains("pmtu")
        || lower.contains("jumbo frame") && lower.contains("test")
        || lower.contains("frame size")
        || lower.contains("mtu discovery")
        || lower.contains("fragmentation")
        || (lower.contains("packet") && lower.contains("size") && lower.contains("max"))
        || (lower.contains("vpn") && lower.contains("mtu"))
        || (lower.contains("mtu") && lower.contains("check"));
    let asks_latency = (lower
        .split_whitespace()
        .any(|w| w.trim_matches(|c: char| !c.is_alphanumeric()) == "ping"))
        || lower.contains("latency")
        || lower.contains("packet loss")
        || lower.contains("rtt")
        || lower.contains("round trip")
        || lower.contains("reachability")
        || lower.contains("ping test")
        || (lower.contains("network") && lower.contains("slow"))
        || (lower.contains("internet") && lower.contains("slow"))
        || (lower.contains("connection") && lower.contains("slow"))
        || (lower.contains("high") && lower.contains("latency"))
        || lower.contains("network lag")
        || lower.contains("jitter");
    let asks_network_adapter = lower.contains("nic settings")
        || lower.contains("nic offload")
        || lower.contains("adapter settings")
        || lower.contains("adapter offload")
        || lower.contains("jumbo frame")
        || lower.contains("rss setting")
        || lower.contains("tcp offload")
        || lower.contains("lso")
        || lower.contains("checksum offload")
        || lower.contains("wake on lan")
        || lower.contains("wake-on-lan")
        || lower.contains("wol")
        || lower.contains("nic advanced")
        || lower.contains("adapter error")
        || lower.contains("duplex mismatch")
        || lower.contains("ethernet not")
        || lower.contains("ethernet port")
        || lower.contains("ethernet cable")
        || lower.contains("wired connection not")
        || lower.contains("wired network not")
        || lower.contains("nic not working")
        || lower.contains("network adapter not")
        || lower.contains("network card")
        || lower.contains("link speed")
        || lower.contains("network adapter settings")
        || (lower.contains("nic")
            && (lower.contains("driver")
                || lower.contains("setting")
                || lower.contains("error")
                || lower.contains("config")));
    let asks_ipv6 = lower.contains("ipv6")
        || lower.contains("slaac")
        || lower.contains("dhcpv6")
        || lower.contains("ipv6 address")
        || lower.contains("ipv6 prefix")
        || lower.contains("ipv6 gateway")
        || lower.contains("ipv6 config")
        || lower.contains("privacy extension")
        || lower.contains("global unicast")
        || lower.contains("link-local address")
        || (lower.contains("ipv6")
            && (lower.contains("check") || lower.contains("show") || lower.contains("status")));
    let asks_tcp_params = lower.contains("tcp autotuning")
        || lower.contains("tcp auto-tuning")
        || lower.contains("tcp window scaling")
        || lower.contains("tcp congestion")
        || lower.contains("congestion algorithm")
        || lower.contains("congestion provider")
        || lower.contains("tcp settings")
        || lower.contains("tcp parameters")
        || lower.contains("tcp tuning")
        || lower.contains("tcp chimney")
        || lower.contains("tcp offload")
        || lower.contains("ecn")
        || lower.contains("rwin")
        || lower.contains("receive window")
        || lower.contains("dynamic port range")
        || lower.contains("tcp window")
        || (lower.contains("tcp")
            && (lower.contains("slow")
                || lower.contains("throughput")
                || lower.contains("performance")
                || lower.contains("config")
                || lower.contains("speed")
                || lower.contains("window size")));
    let asks_wlan_profiles = lower.contains("saved wifi")
        || lower.contains("saved wireless")
        || lower.contains("wifi profile")
        || lower.contains("wlan profile")
        || lower.contains("wireless profile")
        || lower.contains("saved network")
        || lower.contains("known network")
        || lower.contains("netsh wlan")
        || (lower.contains("wifi")
            && (lower.contains("security")
                || lower.contains("audit")
                || lower.contains("wep")
                || lower.contains("saved")
                || lower.contains("remember")
                || lower.contains("password")))
        || (lower.contains("wireless")
            && (lower.contains("profile")
                || lower.contains("saved")
                || lower.contains("audit")
                || lower.contains("remember")));
    let asks_ipsec = lower.contains("ipsec")
        || lower.contains("ip sec")
        || lower.contains("ipsec sa")
        || lower.contains("security association")
        || lower.contains("ike ")
        || lower.contains("ikev2")
        || lower.contains("ike tunnel")
        || lower.contains("ipsec tunnel")
        || lower.contains("ipsec policy")
        || lower.contains("ipsec rule")
        || lower.contains("policy agent")
        || lower.contains("xfrm")
        || (lower.contains("ipsec")
            && (lower.contains("check") || lower.contains("active") || lower.contains("status")));
    let asks_netbios = lower.contains("netbios")
        || lower.contains("nbtstat")
        || lower.contains("wins server")
        || lower.contains("wins address")
        || lower.contains("netbios name")
        || lower.contains("netbios over tcp")
        || lower.contains("nbns")
        || (lower.contains("wins")
            && (lower.contains("server") || lower.contains("config") || lower.contains("check")));
    let asks_nic_teaming = lower.contains("nic team")
        || lower.contains("nic teaming")
        || lower.contains("network team")
        || lower.contains("lacp")
        || lower.contains("link aggregation")
        || lower.contains("bonding")
        || lower.contains("bond interface")
        || lower.contains("lbfo")
        || (lower.contains("team")
            && (lower.contains("nic") || lower.contains("adapter") || lower.contains("network")))
        || (lower.contains("bond")
            && (lower.contains("adapter") || lower.contains("interface") || lower.contains("nic")));
    let asks_snmp = lower.contains("snmp")
        || lower.contains("snmp agent")
        || lower.contains("snmp trap")
        || lower.contains("community string")
        || lower.contains("community name")
        || lower.contains("snmp service")
        || lower.contains("snmpd");
    let asks_port_test = lower.contains("port test")
        || lower.contains("test port")
        || lower.contains("port check")
        || lower.contains("check port")
        || lower.contains("port reachab")
        || lower.contains("can i reach")
        || lower.contains("is port")
        || lower.contains("tcp test")
        || lower.contains("test-netconnection")
        || lower.contains("test connection")
        || (lower.contains("port")
            && (lower.contains("open")
                || lower.contains("closed")
                || lower.contains("blocked")
                || lower.contains("reachable")))
        || (lower.contains("reach") && lower.contains("port"));
    let asks_network_profile = lower.contains("network profile")
        || lower.contains("network location")
        || lower.contains("network category")
        || lower.contains("public network")
        || lower.contains("private network")
        || lower.contains("domain network")
        || lower.contains("net profile")
        || (lower.contains("network") && lower.contains("location"))
        || (lower.contains("firewall") && lower.contains("profile") && lower.contains("network"));
    let asks_dns_lookup = lower.contains("dns lookup")
        || lower.contains("dns record")
        || lower.contains("nslookup")
        || lower.contains("resolve-dnsname")
        || lower.contains("gethostaddresses")
        || lower.contains("gethostentry")
        || lower.contains("[system.net.dns]")
        || lower.contains("net.dns]")
        || lower.contains("look up ")
        || lower.contains("look up the")
        || lower.contains("resolve ")
        || lower.contains("mx record")
        || lower.contains("srv record")
        || lower.contains("txt record")
        || lower.contains("a record")
        || lower.contains("aaaa record")
        || lower.contains("cname record")
        || lower.contains(" dig ")
        || lower.starts_with("host ")
        || (lower.contains("what") && lower.contains("ip") && lower.contains("for"))
        || (lower.contains("ip address") && lower.contains(" of "))
        || (lower.contains("resolve")
            && (lower.contains("hostname") || lower.contains("domain") || lower.contains("name")))
        || (lower.contains("lookup")
            && (lower.contains("domain") || lower.contains("host") || lower.contains("name")));
    let asks_peripherals = lower.contains("peripheral")
        || lower.contains("usb")
        || lower.contains("keyboard")
        || lower.contains("mouse")
        || lower.contains("pointer")
        || lower.contains("monitor")
        || lower.contains("input device")
        || lower.contains("connected hardware");
    let asks_sessions = lower.contains("session")
        || lower.contains("login")
        || lower.contains("who is on")
        || lower.contains("active user")
        || lower.contains("who is logged on")
        || lower.contains("who is logged in")
        || lower.contains("logged on users")
        || lower.contains("logged in users")
        || lower.contains("current users")
        || lower.contains("query session")
        || lower.contains("qwinsta")
        || lower.contains("connected users")
        || lower.contains("user sessions")
        || lower.contains("terminal session")
        || (lower.contains("who") && lower.contains("logged"))
        || (lower.contains("who")
            && lower.contains("using")
            && (lower.contains("computer") || lower.contains("machine")));
    let asks_virtualization = lower.contains("virtualization")
        || lower.contains("hypervisor")
        || lower.contains("vt-x")
        || lower.contains("slat")
        || lower.contains("v-p")
        || lower.contains("nested virt")
        || lower.contains("cpu model")
        || lower.contains("ram size")
        || lower.contains("hardware spec")
        || lower.contains("hardware dna")
        || lower.contains("hardware info")
        || lower.contains("bios version")
        || lower.contains("motherboard")
        || lower.contains("how much ram")
        || lower.contains("what processor")
        || lower.contains("what cpu")
        || (lower.contains("what hardware") && lower.contains("have"))
        || (lower.contains("hardware") && lower.contains("inventory"));
    let asks_startup = lower.contains("startup")
        || lower.contains("boot program")
        || lower.contains("autorun")
        || lower.contains("run at boot")
        || lower.contains("startup program")
        || lower.contains("startup app")
        || lower.contains("startup list")
        || lower.contains("startup item")
        || lower.contains("starts with windows")
        || lower.contains("start with windows")
        || lower.contains("launch at startup")
        || lower.contains("launch on startup")
        || lower.contains("open at startup")
        || lower.contains("open on boot")
        || lower.contains("runs on boot")
        || lower.contains("run at login")
        || lower.contains("msconfig")
        || lower.contains("login item")
        || lower.contains("autostart")
        || (lower.contains("disable") && lower.contains("startup"))
        || (lower.contains("what") && lower.contains("start") && lower.contains("boot"))
        || (lower.contains("load") && lower.contains("boot"))
        || (lower.contains("load") && lower.contains("startup") && !lower.contains("reload"));
    let asks_env_doctor = lower.contains("env doctor")
        || lower.contains("environment doctor")
        || lower.contains("package manager")
        || lower.contains("package managers")
        || lower.contains("shims")
        || lower.contains("path drift")
        || lower.contains("environment is broken")
        || lower.contains("env is broken")
        || (lower.contains("dev machine") && lower.contains("off"))
        || (lower.contains("environment") && lower.contains("sane"));
    let asks_lan_discovery = lower.contains("upnp")
        || lower.contains("ssdp")
        || lower.contains("mdns")
        || lower.contains("bonjour")
        || lower.contains("llmnr")
        || lower.contains("network neighborhood")
        || lower.contains("device discovery")
        || lower.contains("local discovery")
        || lower.contains("discover local devices")
        || lower.contains("discover devices")
        || lower.contains("browse computers")
        || (lower.contains("local network")
            && (lower.contains("discover")
                || lower.contains("discovery")
                || lower.contains("neighborhood")
                || lower.contains("device")
                || lower.contains("devices")
                || lower.contains("aware of")))
        || ((lower.contains("netbios") || lower.contains("smb visibility"))
            && !lower.contains("active directory"))
        || ((lower.contains("nas")
            || lower.contains("printer")
            || lower.contains("device")
            || lower.contains("computer")
            || lower.contains("pc"))
            && ((lower.contains("can't") && lower.contains("see"))
                || (lower.contains("cannot") && lower.contains("see"))
                || (lower.contains("cant") && lower.contains("see"))
                || lower.contains("can't see")
                || lower.contains("cannot see")
                || lower.contains("cant see")
                || lower.contains("not visible")
                || lower.contains("not showing up")
                || lower.contains("not show up")
                || lower.contains("discover"))
            && (lower.contains("network")
                || lower.contains("lan")
                || lower.contains("local")
                || lower.contains("neighborhood")));
    let asks_network = (((lower.contains("network") && !lower.contains("active directory"))
        && !lower.contains("stat")
        && !lower.contains("share")
        && !lower.contains("throughput"))
        || lower.contains("adapter")
        || lower.contains("ip address")
        || lower.contains("ipconfig")
        || lower.contains("ipv4")
        || lower.contains("ipv6")
        || lower.contains("subnet")
        || lower.contains("dns server")
        || lower.contains("nameserver")
        || lower.contains("wifi")
        || lower.contains("wireless")
        || lower.contains("ethernet")
        || lower.contains("lan"))
        && !asks_ad_user;
    let asks_services = lower.contains("service")
        || lower.contains("services")
        || lower.contains("daemon")
        || lower.contains("startup type")
        || lower.contains("background service")
        || lower.contains("windows service")
        || lower.contains("systemctl")
        || lower.contains("get-service");
    let asks_processes = lower.contains("process")
        || lower.contains("processes")
        || lower.contains("task manager")
        || lower.contains("what is running")
        || lower.contains("what's running")
        || lower.contains("using my ram")
        || lower.contains("using ram")
        || lower.contains("using my cpu")
        || lower.contains("using the cpu")
        || lower.contains("top memory")
        || lower.contains("top ram")
        || lower.contains("high memory")
        || lower.contains("resource-heavy processes")
        || lower.contains("heavy hitters")
        || lower.contains("cpu hog")
        || lower.contains("memory hog")
        || lower.contains("ram hog")
        || lower.contains("hogging cpu")
        || lower.contains("hogging ram")
        || lower.contains("hogging memory")
        || lower.contains("eating up cpu")
        || lower.contains("eating up ram")
        || lower.contains("eating up memory")
        || lower.contains("eating my cpu")
        || lower.contains("eating my ram")
        || lower.contains("eating my memory")
        || (lower.contains("hogging")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("memory")))
        || (lower.contains("eating up")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("memory")))
        || (lower.contains("using the most")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("memory")))
        || (lower.contains("most cpu")
            || lower.contains("most ram")
            || lower.contains("most memory"))
        || (lower.contains("hitting")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("disk")));
    let asks_toolchains = lower.contains("developer tools")
        || lower.contains("toolchains")
        || lower.contains("toolchain not found")
        || lower.contains("toolchain missing")
        || (lower.contains("installed") && lower.contains("version"))
        || (lower.contains("detect") && lower.contains("version"));
    let asks_permissions = lower.contains("permission")
        || lower.contains("access control")
        || lower.contains("get-acl")
        || lower.contains("acl ")
        || lower.contains("icacls")
        || lower.contains("takeown")
        || lower.contains("ntfs permission")
        || (lower.contains("who has") && lower.contains("access"));
    let asks_login_history = lower.contains("login history")
        || lower.contains("logon history")
        || lower.contains("who logged in")
        || lower.contains("recent logon")
        || lower.contains("failed logon")
        || lower.contains("event id 4624")
        || lower.contains("eventid 4624");
    let asks_registry_audit = lower.contains("registry audit")
        || lower.contains("persistence")
        || lower.contains("debugger hijack")
        || lower.contains("ifeo")
        || lower.contains("winlogon shell")
        || lower.contains("bootexecute")
        || lower.contains("reg query")
        || lower.contains("regedit")
        || lower.contains("sticky keys")
        || lower.contains("sethc.exe");
    let asks_share_access = lower.contains("share access")
        || lower.contains("unc path")
        || lower.contains("smbshare")
        || lower.contains("net share")
        || lower.contains("net use")
        || lower.contains("\\\\")
        || lower.contains("share is reachable")
        || lower.contains("reachable share")
        || (lower.contains("network share")
            && (lower.contains("reach") || lower.contains("access") || lower.contains("test")))
        || (lower.contains("network drive") && !lower.contains("network drives"))
        || lower.contains("mapped drive")
        || lower.contains("shared folder");
    let asks_thermal = lower.contains("thermal")
        || (lower.contains("throttl") && !lower.contains("gpu"))
        || lower.contains("overheat")
        || lower.contains("too hot")
        || lower.contains("running hot")
        || lower.contains("laptop hot")
        || lower.contains("pc getting hot")
        || lower.contains("getting hot")
        || lower.contains("temperature high")
        || lower.contains("cpu temp")
        || lower.contains("cpu temperature")
        || lower.contains("temp sensor")
        || lower.contains("check temps")
        || lower.contains("fan loud")
        || lower.contains("fan noise")
        || lower.contains("fan running")
        || lower.contains("fans running")
        || lower.contains("fan spinning")
        || lower.contains("fans spinning")
        || lower.contains("loud fan")
        || lower.contains("fan always on")
        || (lower.contains("fan") && lower.contains("always on"))
        || lower.contains("fan constantly")
        || lower.contains("fan at max")
        || lower.contains("fan at 100")
        || (lower.contains("temperature")
            && (lower.contains("cpu")
                || lower.contains("gpu")
                || lower.contains("system")
                || lower.contains("sensor")
                || lower.contains("check")
                || lower.contains("monitor")));
    let asks_overclocker = lower.contains("overclocker")
        || lower.contains("nvidia stats")
        || lower.contains("silicon health")
        || lower.contains("mhz")
        || ((lower.contains("voltage") || lower.contains("volts"))
            && (lower.contains("gpu")
                || lower.contains("cpu")
                || lower.contains("nvidia")
                || lower.contains("silicon")))
        || (lower.contains("gpu")
            && (lower.contains("throttl")
                || lower.contains("bottleneck")
                || lower.contains("clock")
                || lower.contains("fan")
                || lower.contains("power draw")
                || lower.contains("frequency")
                || lower.contains("overheating")
                || lower.contains("usage")
                || lower.contains("utilization")
                || lower.contains("performance")));
    let asks_hardware = lower.contains("cpu model")
        || lower.contains("ram size")
        || lower.contains("hardware spec")
        || (lower.contains("what hardware") && lower.contains("have"))
        || (lower.contains("gpu") && (lower.contains("what") || lower.contains("show")))
        || lower.contains("motherboard")
        || lower.contains("bios version")
        || lower.contains("graphics card")
        || lower.contains("video card")
        || lower.contains("system information")
        || lower.contains("system info")
        || lower.contains("system specs")
        || lower.contains("computer spec")
        || lower.contains("display adapter")
        || (lower.contains("how much") && lower.contains("ram"))
        || (lower.contains("what") && lower.contains("processor"))
        || (lower.contains("what") && lower.contains("cpu") && !lower.contains("using"));
    let asks_activation = lower.contains("activation")
        || lower.contains("activated")
        || lower.contains("not activated")
        || lower.contains("product key")
        || lower.contains("license expired")
        || lower.contains("slmgr")
        || lower.contains("license status")
        || lower.contains("is windows genuine")
        || lower.contains("licensed")
        || lower.contains("unlicensed")
        || (lower.contains("activate") && lower.contains("window"));
    let asks_patch_history = lower.contains("patch history")
        || lower.contains("hotfix")
        || lower.contains("kb history")
        || lower.contains("installed updates")
        || lower.contains("security patch")
        || (lower.contains("update") && lower.contains("applied"));
    let asks_ports = lower.contains("listening on port")
        || lower.contains("listening port")
        || lower.contains("open port")
        || lower.contains("port 3000")
        || lower.contains("listening on ")
        || lower.contains("what ports are")
        || lower.contains("what port is")
        || lower.contains("exposed")
        || lower.contains("what is listening")
        || (lower.contains("listening") && lower.contains("port"));
    let asks_repo_doctor = lower.contains("repo doctor")
        || lower.contains("repository doctor")
        || lower.contains("workspace health")
        || lower.contains("repo health")
        || lower.contains("workspace sanity")
        || (lower.contains("git state")
            && (lower.contains("release artifacts")
                || lower.contains("build markers")
                || lower.contains("hematite memory")));
    let asks_directory = lower.contains("directory")
        || lower.contains("folder")
        || lower.contains("how big")
        || lower.contains("biggest");

    let asks_mutation_intent = (lower.contains("make")
        || lower.contains("create")
        || lower.contains("mkdir")
        || lower.contains("organize")
        || lower.contains("edit")
        || lower.contains("write")
        || lower.contains("save")
        || lower.contains("update")
        || lower.contains("change")
        || lower.contains("fix")
        || lower.contains("implement")
        || lower.contains("refactor"))
        && (lower.contains("folder")
            || lower.contains("directory")
            || lower.split_whitespace().any(|w| {
                let w = w.trim_matches(|c: char| !c.is_alphanumeric());
                w == "file"
                    || w == "files"
                    || w == "code"
                    || w == "script"
                    || w == "css"
                    || w == "js"
                    || w == "html"
                    || w == "ts"
                    || w == "rust"
                    || w == "json"
                    || w == "logic"
            })
            || lower.contains("code")
            || lower.contains("desktop")
            || lower.contains("logic")
            || lower.contains("css")
            || lower.contains("styles")
            || lower.contains("script")
            || code_kw_ac().find(&lower).is_some());
    let asks_broad_readiness = lower.contains("local development")
        || lower.contains("ready for local development")
        || (lower.contains("machine") && lower.contains("ready"))
        || (lower.contains("computer") && lower.contains("ready"));
    let asks_os_config = lower.contains("firewall")
        || lower.contains("power plan")
        || lower.contains("power settings")
        || lower.contains("powercfg")
        || lower.contains("uptime")
        || lower.contains("boot time")
        || lower.contains("last boot")
        || lower.contains("windows version")
        || lower.contains("what version of windows")
        || lower.contains("os version")
        || lower.contains("build number")
        || lower.contains("windows build")
        || lower.contains("edition of windows")
        || lower.contains("which windows")
        || (lower.contains("windows")
            && (lower.contains("10 or 11") || lower.contains("11 or 10")));
    let asks_health_report = lower.contains("health report")
        || lower.contains("system health")
        || (lower.contains("how") && lower.contains("machine") && lower.contains("doing"))
        || (lower.contains("status") && lower.contains("report") && !lower.contains("git"));
    let asks_updates = lower.contains("up to date")
        || lower.contains("windows update")
        || lower.contains("pending update")
        || lower.contains("update available")
        || lower.contains("check for update")
        || lower.contains("latest update")
        || (lower.contains("update")
            && (lower.contains("my pc")
                || lower.contains("my computer")
                || lower.contains("my machine")))
        || lower.contains("check updates")
        || lower.contains("update windows")
        || lower.contains("windows needs update")
        || (lower.contains("windows") && lower.contains("out of date"));
    let asks_security = lower.contains("antivirus")
        || lower.contains("defender")
        || lower.contains("virus protection")
        || lower.contains("malware")
        || lower.contains("windows security")
        || lower.contains("windows activated")
        || lower.contains("activation status")
        || (lower.contains("protected") && (lower.contains("pc") || lower.contains("computer")))
        || (lower.contains("security")
            && !lower.contains("git")
            && !lower.contains("ssh")
            && !lower.contains("token"));
    let asks_pending_reboot = lower.contains("need to restart")
        || lower.contains("need to reboot")
        || lower.contains("requires restart")
        || lower.contains("requires a reboot")
        || lower.contains("reboot required")
        || lower.contains("restart required")
        || lower.contains("pending restart")
        || lower.contains("pending reboot")
        || (lower.contains("restart")
            && (lower.contains("waiting")
                || lower.contains("queued")
                || lower.contains("required")))
        || (lower.contains("reboot") && lower.contains("required"))
        || (lower.contains("reboot") && lower.contains("pending"))
        || (lower.contains("restart") && lower.contains("pending"))
        || (lower.contains("have to") && (lower.contains("restart") || lower.contains("reboot")))
        || (lower.contains("do i need to")
            && (lower.contains("restart") || lower.contains("reboot")));
    let asks_disk_health = lower.contains("disk health")
        || lower.contains("drive health")
        || lower.contains("hard drive dying")
        || lower.contains("smart status")
        || lower.contains("drive failing")
        || lower.contains("drive fail")
        || lower.contains("ssd health")
        || lower.contains("nvme health")
        || lower.contains("hard drive status")
        || (lower.contains("drive") && lower.contains("status") && !lower.contains("backup"))
        || (lower.contains("dying") && (lower.contains("drive") || lower.contains("disk")))
        || (lower.contains("healthy")
            && (lower.contains("drive")
                || lower.contains("disk")
                || lower.contains("ssd")
                || lower.contains("hdd")))
        || lower.contains("bad sector")
        || lower.contains("smart data")
        || (lower.contains("disk") && lower.contains("fail"));
    let asks_battery = lower.contains("battery")
        || lower.contains("battery life")
        || lower.contains("battery health")
        || lower.contains("battery wear")
        || lower.contains("charge level")
        || lower.contains("charge percentage")
        || lower.contains("current charge")
        || lower.contains("charge status")
        || lower.contains("how long until")
        || (lower.contains("dying") && lower.contains("batter"));
    let asks_app_crashes = lower.contains("application crash")
        || lower.contains("application error")
        || lower.contains("application hang")
        || lower.contains("app hang")
        || lower.contains("faulting application")
        || lower.contains("faulting module")
        || lower.contains("exception code")
        || lower.contains("windows error reporting")
        || lower.contains("wer report")
        || lower.contains("which app crashed")
        || lower.contains("what app crashed")
        || lower.contains("what crashed")
        || lower.contains("app crash history")
        || lower.contains("application crash log")
        || lower.contains("apps crashing")
        || lower.contains("apps have been crashing")
        || lower.contains("applications crashing")
        || lower.contains("applications have been crashing")
        || lower.contains("what applications crashed")
        || lower.contains("which applications crashed")
        || lower.contains("what applications have been crashing")
        || lower.contains("which applications have been crashing")
        || (lower.contains("applications") && lower.contains("crashing"))
        || (lower.contains("apps") && lower.contains("crashing"))
        || (lower.contains("crash") && lower.contains("program"))
        || (lower.contains("crash")
            && (lower.contains("chrome")
                || lower.contains("edge")
                || lower.contains("firefox")
                || lower.contains("discord")
                || lower.contains("steam")
                || lower.contains("office")
                || lower.contains("word")
                || lower.contains("excel")
                || lower.contains("photoshop")));
    let asks_recent_crashes = lower.contains("crash")
        || lower.contains("bsod")
        || lower.contains("blue screen")
        || lower.contains("why did my pc restart")
        || lower.contains("unexpected restart")
        || lower.contains("sudden restart")
        || lower.contains("keep restarting")
        || lower.contains("keeps restarting")
        || lower.contains("restarts randomly")
        || lower.contains("random restart")
        || lower.contains("random reboot")
        || lower.contains("kernel panic")
        || (lower.contains("restart") && lower.contains("itself"))
        || (lower.contains("restart") && lower.contains("by itself"));
    let asks_log_check = lower.contains("event log")
        || lower.contains("windows log")
        || lower.contains("system log")
        || lower.contains("error log")
        || lower.contains("recent errors")
        || lower.contains("recent warnings")
        || lower.contains("recent events")
        || lower.contains("event viewer")
        || lower.contains("journald")
        || lower.contains("journal log")
        || lower.contains("show me warnings")
        || (lower.contains("log") && lower.contains("error"))
        || (lower.contains("log") && lower.contains("warning"))
        || (lower.contains("show me") && lower.contains("error"))
        || (lower.contains("show me") && lower.contains("warning"))
        || (lower.contains("what errors") && lower.contains("log"));
    let asks_scheduled_tasks = lower.contains("scheduled task")
        || lower.contains("scheduled tasks")
        || lower.contains("task scheduler")
        || lower.contains("what runs on a timer")
        || lower.contains("what runs at")
        || lower.contains("cron job")
        || lower.contains("background task")
        || lower.contains("scheduled job")
        || lower.contains("runs automatically")
        || lower.contains("running automatically")
        || lower.contains("auto-run task")
        || (lower.contains("background") && lower.contains("run"))
        || (lower.contains("periodic") && lower.contains("run"))
        || (lower.contains("what") && lower.contains("schedule"));
    let asks_dev_conflicts = lower.contains("dev conflict")
        || lower.contains("environment conflict")
        || lower.contains("toolchain conflict")
        || lower.contains("version conflict")
        || lower.contains("path conflict")
        || lower.contains("duplicate path")
        || lower.contains("package manager conflict")
        || lower.contains("nvm conflict")
        || lower.contains("pyenv conflict")
        || (lower.contains("python") && lower.contains("wrong version"))
        || (lower.contains("node") && lower.contains("wrong version"))
        || lower.contains("conda shadow")
        || lower.contains("dev environment clean");
    let asks_disk_benchmark = lower.contains("benchmark")
        || lower.contains("stress test")
        || lower.contains("load test")
        || lower.contains("intensity report")
        || lower.contains("io intensity")
        || lower.contains("disk intensity")
        || lower.contains("thrash")
        || lower.contains("latency report");
    let asks_storage_deep = lower.contains("where did my")
        && (lower.contains("space") || lower.contains("disk") || lower.contains("storage"))
        || lower.contains("what is taking up")
        || lower.contains("what is using my disk")
        || lower.contains("what is eating my disk")
        || lower.contains("biggest folders")
        || lower.contains("largest folders")
        || lower.contains("largest directories")
        || lower.contains("what is filling")
        || lower.contains("storage breakdown")
        || lower.contains("disk breakdown")
        || lower.contains("find large files")
        || lower.contains("find big files")
        || lower.contains("storage deep")
        || lower.contains("deep storage")
        || lower.contains("storage analysis")
        || lower.contains("disk analysis")
        || (lower.contains("clean up")
            && (lower.contains("disk")
                || lower.contains("drive")
                || lower.contains("space")
                || lower.contains("storage")))
        || (lower.contains("clean") && lower.contains("c drive"))
        || (lower.contains("analyze") && (lower.contains("storage") || lower.contains("disk")));
    let asks_storage = lower.contains("storage")
        || lower.contains("disk space")
        || lower.contains("drive capacity")
        || lower.contains("free space")
        || lower.contains("how much space")
        || lower.contains("space left")
        || lower.contains("running out of space")
        || lower.contains("i/o pressure")
        || lower.contains("disk usage")
        || lower.contains("disk usage")
        || lower.contains("how much disk")
        || lower.contains("how full")
        || lower.contains("cache size")
        || (lower.contains("drive") && lower.contains("usage"))
        || (lower.contains("drives") && lower.contains("usage"))
        || (lower.contains("where") && lower.contains("space") && lower.contains("go"))
        || ((lower.contains("disk") || lower.contains("drive")) && lower.contains("full"))
        || lower.contains("out of space");
    let asks_resource_load = lower.contains("resource load")
        || lower.contains("system load")
        || lower.contains("performance")
        || lower.contains("utilization")
        || lower.contains("usage report")
        || lower.contains("performance report")
        || lower.contains("what is my load")
        || lower.contains("current load")
        || lower.contains("why is it slow")
        || lower.contains("why is it laggy")
        || lower.contains("memory pressure")
        || lower.contains("memory load")
        || lower.contains("process overhead")
        || lower.contains("slow")
        || lower.contains("lag")
        || lower.contains("sluggish")
        || lower.contains("hang")
        || lower.contains("unresponsive")
        || lower.contains("frozen")
        || lower.contains("freezing up")
        || lower.contains("computer freeze")
        || lower.contains("is it working hard")
        || lower.contains("high cpu")
        || lower.contains("high ram")
        || lower.contains("cpu load")
        || lower.contains("heavy hitters")
        || (lower.contains("resource") && lower.contains("usage"));

    let asks_connectivity = lower.contains("internet")
        || lower.contains("online")
        || lower.contains("connectivity")
        || lower.contains("am i connected")
        || lower.contains("ping google")
        || lower.contains("reach the internet")
        || lower.contains("internet access")
        || lower.contains("no internet")
        || lower.contains("internet down")
        || lower.starts_with("ping ")
        || lower.contains(" ping ")
        || (lower.contains("check") && lower.contains("connection"))
        || (lower.contains("dns") && (lower.contains("resolv") || lower.contains("working")))
        || lower.contains("can't browse")
        || lower.contains("cannot browse")
        || lower.contains("web browsing")
        || lower.contains("browser not loading")
        || lower.contains("pages not loading")
        || lower.contains("websites not loading")
        || lower.contains("no network")
        || (lower.contains("network") && lower.contains("down"))
        || (lower.contains("can't") && lower.contains("connect to internet"))
        || (lower.contains("cannot") && lower.contains("connect to internet"))
        || (lower.contains("network") && lower.contains("not working"))
        || (lower.contains("internet") && lower.contains("not working"));
    let asks_wifi = lower.contains("wi-fi")
        || lower.contains("wifi")
        || lower.contains("wireless")
        || lower.contains("wlan")
        || lower.contains("signal strength")
        || lower.contains("ssid")
        || lower.contains("access point")
        || (lower.contains("wireless") && lower.contains("connect"));
    let asks_connections = lower.contains("tcp connection")
        || lower.contains("active connection")
        || lower.contains("established connection")
        || lower.contains("socket")
        || lower.contains("netstat")
        || lower.contains("outbound connection")
        || lower.contains("inbound connection")
        || lower.contains("remote connection")
        || lower.contains("connection list")
        || (lower.contains("connection") && lower.contains("active"))
        || (lower.contains("connection") && lower.contains("open"))
        || (lower.contains("what") && lower.contains("connecting"))
        || (lower.contains("which") && lower.contains("connecting"))
        || (lower.contains("process") && lower.contains("network") && lower.contains("connect"));
    let asks_vpn = lower.contains("vpn")
        || lower.contains("virtual private network")
        || lower.contains("wireguard")
        || lower.contains("anyconnect")
        || lower.contains("globalprotect")
        || lower.contains("pulse secure")
        || lower.contains("openvpn")
        || lower.contains("split tunnel")
        || lower.contains("vpn adapter")
        || (lower.contains("tunnel") && (lower.contains("network") || lower.contains("vpn")));
    let asks_proxy = lower.contains("proxy")
        || lower.contains("proxy setting")
        || lower.contains("winhttp proxy")
        || lower.contains("system proxy")
        || (lower.contains("routed") && lower.contains("proxy"));
    let asks_firewall_rules = (lower.contains("firewall")
        && (lower.contains("rule")
            || lower.contains("block")
            || lower.contains("allow")
            || lower.contains("inbound")
            || lower.contains("outbound")))
        || lower.contains("blocked port")
        || lower.contains("firewall rule");
    let asks_traceroute = lower.contains("traceroute")
        || lower.contains("tracert")
        || lower.contains("tracepath")
        || lower.contains("trace route")
        || lower.contains("trace the route")
        || lower.contains("trace the path")
        || lower.contains("network path")
        || lower.contains("how many hops")
        || lower.contains("where does traffic go")
        || (lower.contains("trace") && lower.contains("hop"))
        || (lower.contains("route") && lower.contains("traffic"))
        || (lower.contains("trace") && lower.contains("8.8.8.8"))
        || (lower.contains("path") && lower.contains("8.8.8.8"));
    let asks_dns_cache = lower.contains("dns cache")
        || lower.contains("cached dns")
        || lower.contains("dns lookup cache")
        || lower.contains("displaydns")
        || lower.contains("/displaydns")
        || lower.contains("get-dnsclientcache")
        || lower.contains("dns entries")
        || (lower.contains("dns") && lower.contains("cached"));
    let asks_arp = lower.contains("arp -")
        || lower.contains("arp table")
        || lower.contains("arp cache")
        || lower.contains("mac address")
        || lower.contains("neighbor table")
        || lower.contains("ip to mac")
        || lower.contains("ip neigh")
        || (lower.contains("arp")
            && (lower.contains("who") || lower.contains("entry") || lower.contains("entries")));
    let asks_route_table = lower.contains("route print")
        || lower.contains("route table")
        || lower.contains("routing table")
        || lower.contains("get-netroute")
        || lower.contains("default gateway")
        || lower.contains("network routes")
        || lower.contains("ip route")
        || lower.contains("next hop")
        || (lower.contains("route")
            && (lower.contains("table") || lower.contains("entry") || lower.contains("entries")));
    let asks_env = (lower.contains("environment variable")
        || lower.contains("env var")
        || lower.contains("env vars")
        || lower.contains("show env")
        || lower.contains("list env"))
        && !lower.contains("env doctor");
    let asks_hosts_file = lower.contains("hosts file")
        || lower.contains("/etc/hosts")
        || lower.contains("etc/hosts")
        || lower.contains("hosts entry")
        || lower.contains("hosts entries")
        || (lower.contains("hosts")
            && (lower.contains("redirect")
                || lower.contains("block")
                || lower.contains("loopback")));
    let asks_docker = lower.contains("docker")
        || lower.contains("container")
        || lower.contains("docker compose")
        || lower.contains("docker ps")
        || lower.contains("running container");
    let asks_docker_filesystems = (lower.contains("docker")
        || lower.contains("container")
        || lower.contains("compose")
        || lower.contains("volume")
        || lower.contains("bind mount"))
        && (lower.contains("mount")
            || lower.contains("volume")
            || lower.contains("bind")
            || lower.contains("filesystem")
            || lower.contains("storage")
            || lower.contains("path")
            || lower.contains("missing"));
    let asks_wsl = lower.contains("wsl")
        || lower.contains("windows subsystem")
        || lower.contains("linux distro")
        || lower.contains("ubuntu on windows")
        || (lower.contains("subsystem") && lower.contains("linux"));
    let asks_wsl_filesystems = (lower.contains("wsl")
        || lower.contains("windows subsystem")
        || lower.contains("linux distro")
        || lower.contains("ubuntu on windows")
        || (lower.contains("subsystem") && lower.contains("linux")))
        && (lower.contains("mount")
            || lower.contains("filesystem")
            || lower.contains("storage")
            || lower.contains("disk")
            || lower.contains("vhdx")
            || lower.contains("path bridge")
            || lower.contains("/mnt/c")
            || lower.contains("wsl df")
            || lower.contains("wsl du")
            || lower.contains("du -sh /mnt/c"));
    let asks_ssh = (lower.contains("ssh") && !lower.contains("ssh key") && !lower.contains("git"))
        || lower.contains("sshd")
        || lower.contains("ssh config")
        || lower.contains("ssh server")
        || lower.contains("ssh client")
        || lower.contains("known_hosts")
        || lower.contains("authorized_keys")
        || lower.contains("ssh key")
        || (lower.contains("ssh")
            && (lower.contains("running")
                || lower.contains("service")
                || lower.contains("port 22")));
    let asks_installed_software = lower.contains("installed software")
        || lower.contains("installed program")
        || lower.contains("installed app")
        || lower.contains("installed package")
        || lower.contains("what is installed")
        || lower.contains("what's installed")
        || lower.contains("winget list")
        || lower.contains("list programs")
        || lower.contains("list applications")
        || lower.contains("list all apps")
        || lower.contains("list apps")
        || lower.contains("show applications")
        || lower.contains("show programs")
        || (lower.contains("list") && lower.contains("application"))
        || (lower.contains("show") && lower.contains("application"))
        || (lower.contains("installed")
            && (lower.contains("on this machine")
                || lower.contains("on my machine")
                || lower.contains("on my pc")));
    let asks_databases = lower.contains("postgres")
        || lower.contains("postgresql")
        || lower.contains("mysql")
        || lower.contains("mariadb")
        || lower.contains("mongodb")
        || lower.contains("mongo")
        || lower.contains("redis")
        || lower.contains("sql server")
        || lower.contains("mssql")
        || lower.contains("sqlite")
        || lower.contains("elasticsearch")
        || lower.contains("cassandra")
        || lower.contains("couchdb")
        || (lower.contains("database")
            && (lower.contains("running")
                || lower.contains("service")
                || lower.contains("installed")
                || lower.contains("up")
                || lower.contains("local")))
        || lower.contains("db service")
        || lower.contains("database server")
        || (lower.contains("is")
            && lower.contains("running")
            && (lower.contains("db") || lower.contains("database")));
    let asks_git_config = (lower.contains("git config")
        || lower.contains("git configuration")
        || lower.contains("git global")
        || (lower.contains("git") && lower.contains("user.name"))
        || (lower.contains("git") && lower.contains("user.email"))
        || (lower.contains("git") && lower.contains("signing"))
        || (lower.contains("git") && lower.contains("credential"))
        || (lower.contains("git") && lower.contains("auth"))
        || lower.contains("git push denied")
        || lower.contains("git clone failed")
        || lower.contains("git identity")
        || lower.contains("git aliases"))
        && !lower.contains("github");
    let asks_audit_policy = lower.contains("audit policy")
        || lower.contains("auditpol")
        || lower.contains("audit log")
        || lower.contains("what is being logged")
        || lower.contains("security audit")
        || lower.contains("logon event")
        || lower.contains("login event")
        || lower.contains("audit category")
        || lower.contains("event auditing")
        || (lower.contains("audit") && lower.contains("event"))
        || (lower.contains("what") && lower.contains("being audited"))
        || (lower.contains("audit") && lower.contains("enable"));
    let asks_shares = lower.contains("smb share")
        || lower.contains("network share")
        || lower.contains("shared folder")
        || lower.contains("mapped drive")
        || lower.contains("mapped network drive")
        || lower.contains("get-smbshare")
        || lower.contains("what is shared")
        || lower.contains("what am i sharing")
        || lower.contains("file sharing")
        || lower.contains("smb session")
        || lower.contains("lanmanager")
        || lower.contains("netlanmanager")
        || lower.contains("smb1")
        || lower.contains("smb signing")
        || lower.contains("nfs export")
        || (lower.contains("folder") && lower.contains("shared"))
        || (lower.contains("sharing") && lower.contains("network"))
        || lower.contains("what am i sharing");
    let asks_dns_servers = (lower.contains("dns server")
        || lower.contains("dns resolver")
        || lower.contains("nameserver")
        || lower.contains("which dns")
        || lower.contains("what dns")
        || lower.contains("dns over https")
        || lower.contains("doh")
        || lower.contains("dns search suffix")
        || lower.contains("configured dns")
        || lower.contains("get-dnsclientserveraddress"))
        && !lower.contains("dns cache")
        && (!lower.contains("adapter")
            || contains_any(
                &lower,
                &[
                    "dns server",
                    "dns resolver",
                    "nameserver",
                    "configured dns",
                    "per adapter",
                    "which dns",
                    "what dns",
                    "get-dnsclientserveraddress",
                ],
            ))
        && !lower.contains("ip address")
        && !lower.contains("gateway");
    let asks_bitlocker = lower.contains("bitlocker")
        || (lower.contains("drive") && lower.contains("encrypt"))
        || (lower.contains("disk") && lower.contains("encrypt"))
        || (lower.contains("ssd") && lower.contains("encrypt"))
        || (lower.contains("volume") && lower.contains("encrypt"))
        || (lower.contains("machine") && lower.contains("encrypt"))
        || lower.contains("encryption status")
        || lower.contains("full disk encryption")
        || lower.contains("drive encryption");
    let asks_rdp = lower.contains("rdp")
        || lower.contains("remote desktop")
        || (lower.contains("remote") && lower.contains("access") && !lower.contains("git"));
    let asks_shadow_copies = lower.contains("shadow copy")
        || lower.contains("shadow copies")
        || lower.contains("vss")
        || lower.contains("snapshot")
        || lower.contains("restore point");
    let asks_pagefile = lower.contains("pagefile")
        || lower.contains("page file")
        || lower.contains("virtual memory")
        || lower.contains("swap file")
        || lower.contains("swap space")
        || lower.contains("memory swapping")
        || lower.contains("paging file")
        || (lower.contains("paging") && lower.contains("file"))
        || (lower.contains("paging") && lower.contains("active"));
    let asks_windows_features = (lower.contains("window") && lower.contains("feature"))
        || lower.contains("optional feature")
        || lower.contains("iis")
        || lower.contains("hyper-v")
        || (lower.contains("feature")
            && (lower.contains("install")
                || lower.contains("enabled")
                || lower.contains("turn on")));
    let asks_printers = lower.contains("printer")
        || lower.contains("print queue")
        || lower.contains("get-printer")
        || lower.contains("printing")
        || lower.contains("can't print")
        || lower.contains("cannot print")
        || lower.contains("print job")
        || lower.contains("print driver")
        || lower.contains("default printer")
        || lower.contains("add printer")
        || lower.contains("print to pdf")
        || (lower.contains("print") && lower.contains("not working"))
        || (lower.contains("print") && lower.contains("stuck"))
        || (lower.contains("print") && lower.contains("pending"))
        || (lower.contains("print") && lower.contains("offline"));
    let asks_winrm = lower.contains("winrm")
        || lower.contains("psremoting")
        || (lower.contains("ps") && lower.contains("remoting"))
        || (lower.contains("remote") && lower.contains("management") && !lower.contains("rdp"));
    let asks_network_stats = (lower.contains("network") && lower.contains("stat"))
        || (lower.contains("adapter")
            && lower.contains("stat")
            && !lower.contains("wlan")
            && !lower.contains("wireless"))
        || (lower.contains("nic") && lower.contains("stat"))
        || lower.contains("throughput")
        || lower.contains("dropped packet")
        || (lower.contains("network") && lower.contains("usage"))
        || (lower.contains("data") && lower.contains("transferred"))
        || (lower.contains("bytes") && lower.contains("transferred"))
        || lower.contains("network traffic")
        || (lower.contains("packet") && lower.contains("error"));
    let asks_udp_ports = lower.contains("udp port")
        || lower.contains("udp listener")
        || (lower.contains("udp")
            && (lower.contains("port")
                || lower.contains("listen")
                || lower.contains("open")
                || lower.contains("service")
                || lower.contains("connection")));

    let asks_domain_health = lower.contains("domain health")
        || lower.contains("dc connectivity")
        || lower.contains("dc reachab")
        || lower.contains("can reach dc")
        || lower.contains("ldap port")
        || lower.contains("kerberos health")
        || lower.contains("kerberos connectivity")
        || lower.contains("ad connectivity")
        || lower.contains("active directory health")
        || lower.contains("domain controller connectivity")
        || lower.contains("domain controller reachab")
        || lower.contains("nltest")
        || lower.contains("dsgetdc")
        || lower.contains("gpo refresh")
        || (lower.contains("domain controller")
            && (lower.contains("reach")
                || lower.contains("connect")
                || lower.contains("test")
                || lower.contains("check")
                || lower.contains("online")
                || lower.contains("up")))
        || (lower.contains("active directory")
            && (lower.contains("connect")
                || lower.contains("reach")
                || lower.contains("health")
                || lower.contains("working")
                || lower.contains("accessible")))
        || (lower.contains("can reach") && lower.contains("domain"))
        || (lower.contains("kerberos") && lower.contains("issue"))
        || (lower.contains("kerberos") && lower.contains("fail"));
    let asks_service_dependencies = lower.contains("service depend")
        || lower.contains("services depend")
        || lower.contains("depends on")
        || lower.contains("service graph")
        || lower.contains("which services depend")
        || lower.contains("what depends on")
        || lower.contains("restart cascade")
        || lower.contains("svc dep")
        || lower.contains("service prerequisite")
        || (lower.contains("service")
            && (lower.contains("dependency")
                || lower.contains("dependencies")
                || lower.contains("required by")
                || lower.contains("needed by")
                || lower.contains("requirement")
                || lower.contains("relationship")));
    let asks_wmi_health = lower.contains("wmi health")
        || lower.contains("wmi corrupt")
        || lower.contains("wmi repository")
        || lower.contains("wmi broken")
        || lower.contains("winmgmt")
        || lower.contains("wmi query fail")
        || lower.contains("wmi not working")
        || (lower.contains("wmi")
            && (lower.contains("health")
                || lower.contains("status")
                || lower.contains("repair")
                || lower.contains("reset")
                || lower.contains("broken")));
    let asks_local_security_policy = lower.contains("password policy")
        || lower.contains("account lockout")
        || lower.contains("lockout policy")
        || lower.contains("lockout threshold")
        || lower.contains("lm compatibility")
        || lower.contains("ntlm level")
        || lower.contains("ntlm policy")
        || lower.contains("local security policy")
        || lower.contains("account policy")
        || lower.contains("uac level")
        || lower.contains("uac policy")
        || lower.contains("uac disabled")
        || lower.contains("uac prompt")
        || lower.contains("uac not")
        || lower.contains("user account control")
        || lower.contains("needs elevation")
        || lower.contains("needs admin")
        || lower.contains("run as administrator")
        || lower.contains("administrator permission")
        || lower.contains("lmcompatibilitylevel")
        || lower.contains("net accounts")
        || lower.contains("lockout")
        || (lower.contains("uac")
            && (lower.contains("off")
                || lower.contains("status")
                || lower.contains("check")
                || lower.contains("on")))
        || (lower.contains("password")
            && (lower.contains("minimum")
                || lower.contains("maximum age")
                || lower.contains("complexity")
                || lower.contains("history")
                || lower.contains("policy")));
    let asks_usb_history = lower.contains("usb history")
        || lower.contains("usb devices connected")
        || lower.contains("usb forensic")
        || lower.contains("usbstor")
        || lower.contains("usb registry")
        || lower.contains("what usb")
        || lower.contains("ever connected usb")
        || lower.contains("usb devices ever")
        || lower.contains("usb drives ever")
        || (lower.contains("usb")
            && (lower.contains("history")
                || lower.contains("forensic")
                || lower.contains("audit")
                || lower.contains("ever connected")
                || lower.contains("registry")
                || lower.contains("plugged")
                || lower.contains("were connected")
                || lower.contains("have been connected")
                || lower.contains("has been connected")));
    let asks_print_spooler = lower.contains("print spooler")
        || lower.contains("spooler service")
        || lower.contains("printnightmare")
        || lower.contains("print nightmar")
        || lower.contains("cve-2021-34527")
        || lower.contains("cve-2021-1675")
        || lower.contains("print security")
        || lower.contains("printer security")
        || lower.contains("printer service")
        || lower.contains("point and print")
        || lower.contains("rpcauthnlevel")
        || (lower.contains("print") && lower.contains("vulnerab"))
        || lower.contains("print service")
        || (lower.contains("printer") && lower.contains("spooler"))
        || (lower.contains("spooler")
            && (lower.contains("status")
                || lower.contains("running")
                || lower.contains("security")
                || lower.contains("hardening")));

    // Host-remediation queries (e.g., "fix cargo not found on this machine") contain
    // code keywords like "cargo" that also trip the mutation guard. Check fix_plan
    // first so these read-only host inspection requests are never silently dropped.
    if asks_fix_plan && asks_mutation_intent {
        return Some("fix_plan");
    }

    // If the user has a clear mutation intent (create folder, edit file),
    // we should NOT route to a read-only host inspection topic, as that would
    // trigger a pre-run crash. The main LLM turn will handle the mutation.
    if asks_mutation_intent {
        return None;
    }

    // Priority 1: High-Precision Enterprise Triage (IT Pro Plus)
    if asks_overclocker {
        Some("overclocker")
    } else if asks_ad_user {
        Some("ad_user")
    } else if asks_user_accounts {
        Some("user_accounts")
    } else if asks_dns_lookup {
        Some("dns_lookup")
    } else if asks_event_query {
        Some("event_query")
    } else if asks_mdm {
        Some("mdm_enrollment")
    } else if asks_hyperv {
        Some("hyperv")
    } else if asks_ip_config {
        Some("ip_config")
    } else if asks_disk_benchmark {
        Some("disk_benchmark")
    } else if asks_fix_plan {
        Some("fix_plan")
    } else if asks_env_doctor {
        Some("env_doctor")
    } else if asks_traceroute {
        Some("traceroute")
    } else if asks_dhcp {
        Some("dhcp")
    } else if asks_mtu {
        Some("mtu")
    } else if asks_ipv6 {
        Some("ipv6")
    } else if asks_domain_health {
        Some("domain_health")
    } else if asks_service_dependencies {
        Some("service_dependencies")
    } else if asks_wmi_health {
        Some("wmi_health")
    } else if asks_local_security_policy {
        Some("local_security_policy")
    } else if asks_usb_history {
        Some("usb_history")
    } else if asks_print_spooler {
        Some("print_spooler")
    } else if asks_latency {
        Some("latency")
    } else if asks_nic_teaming {
        Some("nic_teaming")
    } else if asks_network_stats {
        Some("network_stats")
    } else if asks_share_access {
        Some("share_access")
    } else if asks_thermal {
        Some("thermal")
    } else if asks_activation {
        Some("activation")
    } else if asks_patch_history {
        Some("patch_history")
    } else if asks_bluetooth {
        Some("bluetooth")
    } else if asks_audio {
        Some("audio")
    } else if asks_camera {
        Some("camera")
    } else if asks_identity_auth {
        Some("identity_auth")
    } else if asks_sign_in {
        Some("sign_in")
    } else if asks_installer_health {
        Some("installer_health")
    } else if asks_teams {
        Some("teams")
    } else if asks_windows_backup {
        Some("windows_backup")
    } else if asks_onedrive {
        Some("onedrive")
    } else if asks_browser_health {
        Some("browser_health")
    } else if asks_outlook {
        Some("outlook")
    } else if asks_search_index {
        Some("search_index")
    } else if asks_display_config {
        Some("display_config")
    } else if asks_ntp {
        Some("ntp")
    } else if asks_cpu_power {
        Some("cpu_power")
    } else if asks_credentials {
        Some("credentials")
    } else if asks_tpm {
        Some("tpm")
    } else if asks_network_adapter {
        Some("network_adapter")
    } else if asks_tcp_params {
        Some("tcp_params")
    } else if asks_wlan_profiles {
        Some("wlan_profiles")
    } else if asks_ipsec {
        Some("ipsec")
    } else if asks_udp_ports {
        Some("udp_ports")
    } else if asks_port_test {
        Some("port_test")
    } else if asks_netbios {
        Some("netbios")
    } else if asks_snmp {
        Some("snmp")
    } else if asks_network_profile {
        Some("network_profile")
    } else if asks_permissions {
        Some("permissions")
    } else if asks_login_history {
        Some("login_history")
    } else if asks_registry_audit {
        Some("registry_audit")
    } else if asks_docker_filesystems {
        Some("docker_filesystems")
    } else if asks_wsl_filesystems {
        Some("wsl_filesystems")
    } else if asks_lan_discovery {
        Some("lan_discovery")
    } else if asks_storage_spaces {
        Some("storage_spaces")
    } else if asks_defender_quarantine {
        Some("defender_quarantine")
    } else if asks_storage_deep {
        Some("storage_deep")
    } else if asks_storage {
        Some("storage")
    } else if asks_gpo {
        Some("gpo")
    } else if asks_data_audit {
        Some("data_audit")
    } else if asks_certificates {
        Some("certificates")
    } else if asks_integrity {
        Some("integrity")
    } else if asks_domain {
        Some("domain")
    } else if asks_device_health {
        Some("device_health")
    } else if asks_drivers {
        Some("drivers")
    } else if asks_peripherals {
        Some("peripherals")
    } else if asks_sessions {
        Some("sessions")
    } else if asks_virtualization {
        Some("hardware")
    } else if asks_services {
        Some("services")
    } else if asks_startup {
        Some("startup_items")
    } else if asks_bitlocker {
        Some("bitlocker")
    } else if asks_rdp {
        Some("rdp")
    } else if asks_shadow_copies {
        Some("shadow_copies")
    } else if asks_pagefile {
        Some("pagefile")
    } else if asks_windows_features {
        Some("windows_features")
    } else if asks_printers {
        Some("printers")
    } else if asks_winrm {
        Some("winrm")
    } else if (asks_path && asks_toolchains)
        || (mentions_host_inspection_question(&lower) && asks_broad_readiness)
    {
        Some("summary")
    } else if asks_dns_servers {
        Some("dns_servers")
    } else if asks_connectivity {
        Some("connectivity")
    } else if asks_wifi {
        Some("wifi")
    } else if asks_connections {
        Some("connections")
    } else if asks_vpn {
        Some("vpn")
    } else if asks_proxy {
        Some("proxy")
    } else if asks_firewall_rules {
        Some("firewall_rules")
    } else if asks_dns_cache {
        Some("dns_cache")
    } else if asks_arp {
        Some("arp")
    } else if asks_route_table {
        Some("route_table")
    } else if asks_shares {
        Some("shares")
    } else if asks_network {
        Some("network")
    } else if asks_health_report {
        Some("health_report")
    } else if asks_os_config {
        Some("os_config")
    } else if asks_hardware || asks_virtualization {
        Some("hardware")
    } else if asks_updates {
        Some("updates")
    } else if asks_audit_policy {
        Some("audit_policy")
    } else if asks_security {
        Some("security")
    } else if asks_pending_reboot {
        Some("pending_reboot")
    } else if asks_disk_health {
        Some("disk_health")
    } else if asks_battery {
        Some("battery")
    } else if asks_app_crashes {
        Some("app_crashes")
    } else if asks_recent_crashes {
        Some("recent_crashes")
    } else if asks_log_check {
        Some("log_check")
    } else if asks_scheduled_tasks {
        Some("scheduled_tasks")
    } else if asks_dev_conflicts {
        Some("dev_conflicts")
    } else if asks_databases {
        Some("databases")
    } else if asks_docker {
        Some("docker")
    } else if asks_wsl {
        Some("wsl")
    } else if asks_ssh {
        Some("ssh")
    } else if asks_git_config {
        Some("git_config")
    } else if asks_installed_software {
        Some("installed_software")
    } else if asks_env {
        Some("env")
    } else if asks_hosts_file {
        Some("hosts_file")
    } else if asks_ports {
        Some("ports")
    } else if asks_processes {
        Some("processes")
    } else if asks_repo_doctor {
        Some("repo_doctor")
    } else if lower.contains("desktop")
        && (lower.contains("show")
            || lower.contains("list")
            || lower.contains("what is in")
            || lower.contains("what's in")
            || lower.contains("folder"))
    {
        Some("desktop")
    } else if lower.contains("downloads")
        && (lower.contains("show")
            || lower.contains("list")
            || lower.contains("what is in")
            || lower.contains("what's in")
            || lower.contains("folder"))
    {
        Some("downloads")
    } else if asks_path {
        Some("path")
    } else if asks_toolchains {
        Some("toolchains")
    } else if asks_resource_load {
        Some("resource_load")
    } else if asks_directory {
        Some("directory")
    } else if mentions_host_inspection_question(&lower) && !is_conversational_advisory(&lower) {
        Some("summary")
    } else {
        None
    }
}

type TopicDetector = (&'static str, fn(&str) -> bool);

pub fn all_host_inspection_topics(user_input: &str) -> Vec<&'static str> {
    // All topic detectors in priority order — ordered so more specific topics come
    // before generic fallbacks (e.g. traceroute before network).
    let lower = user_input.to_lowercase();
    let mut topics: Vec<&'static str> = Vec::with_capacity(4);

    let detectors: &[TopicDetector] = &[
        ("overclocker", |l| {
            l.contains("overclocker")
                || l.contains("gpu clock")
                || l.contains("gpu throttle")
                || l.contains("throttle reason")
                || l.contains("root cause")
                || l.contains("nvidia stats")
                || l.contains("silicon health")
                || ((l.contains("voltage") || l.contains("volts"))
                    && (l.contains("gpu")
                        || l.contains("cpu")
                        || l.contains("nvidia")
                        || l.contains("silicon")))
                || (l.contains("gpu")
                    && (l.contains("throttle")
                        || l.contains("bottleneck")
                        || l.contains("performance")
                        || l.contains("overheating")
                        || l.contains("usage")
                        || l.contains("utilization")))
        }),
        ("data_audit", |l| {
            l.contains("data audit")
                || l.contains("audit data")
                || l.contains("csv schema")
                || l.contains("data schema")
                || l.contains("inspect file")
                || l.contains("profile data")
                || l.contains("data distribution")
                || (l.contains("schema") && (l.contains("csv") || l.contains("json")))
        }),
        ("directory", |l| {
            (l.contains("make")
                || l.contains("create")
                || l.contains("mkdir")
                || l.contains("organize"))
                && (l.contains("folder")
                    || l.contains("directory")
                    || l.contains("project area")
                    || l.contains("desktop"))
        }),
        ("ad_user", |l| {
            l.contains("ad user")
                || l.contains("domain user")
                || (l.contains("user") && (l.contains("sid") || l.contains("membership")))
        }),
        ("dns_lookup", |l| {
            l.contains("dns lookup")
                || l.contains("dns record")
                || l.contains("dns query")
                || l.contains("nslookup")
                || l.contains("resolve-dnsname")
                || l.contains("gethostaddresses")
                || l.contains("gethostentry")
                || l.contains("[system.net.dns]")
                || l.contains(" dig ")
                || l.starts_with("host ")
                || (l.contains("ip address") && l.contains(" of "))
                || l.contains("srv record")
                || l.contains("mx record")
        }),
        ("mdm_enrollment", |l| {
            l.contains("mdm")
                || l.contains("intune")
                || l.contains("autopilot")
                || l.contains("device enrollment")
                || l.contains("enrolled in")
                || l.contains("mdm enrollment")
                || l.contains("device management")
                || l.contains("managed device")
                || l.contains("azure ad join")
                || l.contains("aad join")
                || (l.contains("enrolled") && l.contains("device"))
                || (l.contains("enroll") && l.contains("device"))
                || (l.contains("microsoft") && l.contains("endpoint"))
        }),
        ("hyperv", |l| {
            l.contains("hyper-v")
                || l.contains("hyperv")
                || l.contains("hyper v")
                || l.contains("virtual machine")
                || l.contains("running vms")
                || l.contains("list vms")
                || l.contains("list vm")
                || l.contains("vmmem")
                || l.contains("vmms")
                || (l.contains("vm")
                    && !l.contains("nvme")
                    && (l.contains("checkpoint")
                        || l.contains("snapshot")
                        || l.contains("switch")
                        || l.contains("running")))
        }),
        ("ip_config", |l| {
            l.contains("ipconfig")
                || l.contains("ip config")
                || l.contains("adapter detail")
                || l.contains("dhcp lease")
        }),
        ("event_query", |l| {
            l.contains("event id")
                || l.contains("event_id")
                || l.contains("eventid")
                || l.contains("event log query")
                || l.contains("search event")
                || l.contains("query event")
                || l.contains("failed logon event")
                || l.contains("failed login event")
                || l.contains("application error event")
                || ((l.contains("event log")
                    || l.contains("system log")
                    || l.contains("application log")
                    || l.contains("security log"))
                    && (l.contains("last ")
                        || l.contains("past ")
                        || l.contains("today")
                        || l.contains("hour")
                        || l.contains("hours"))
                    && (l.contains("error")
                        || l.contains("errors")
                        || l.contains("warning")
                        || l.contains("warnings")
                        || l.contains("critical")))
                || (l.contains("event")
                    && (l.contains("4625")
                        || l.contains("7034")
                        || l.contains("7031")
                        || l.contains("4648")))
        }),
        ("fix_plan", |l| {
            l.contains("fix")
                && (l.contains("cargo")
                    || l.contains("port ")
                    || l.contains("lm studio")
                    || l.contains("toolchain"))
        }),
        ("updates", |l| {
            l.contains("up to date")
                || l.contains("windows update")
                || l.contains("pending update")
                || l.contains("update available")
                || l.contains("check updates")
                || l.contains("update windows")
                || (l.contains("windows") && l.contains("out of date"))
        }),
        ("security", |l| {
            l.contains("antivirus")
                || l.contains("defender")
                || l.contains("uac")
                || (l.contains("security") && !l.contains("git") && !l.contains("ssh"))
        }),
        ("defender_quarantine", |l| {
            l.contains("defender quarantine")
                || l.contains("quarantine threat")
                || l.contains("quarantine")
                || l.contains("threat history")
                || l.contains("malware history")
                || l.contains("defender history")
                || l.contains("detected threat")
                || l.contains("detected virus")
                || l.contains("malware detected")
                || l.contains("defender found")
                || l.contains("defender detected")
                || l.contains("defender find")
                || l.contains("virus found")
                || l.contains("threats found")
                || l.contains("threat detected")
                || l.contains("threat detection")
                || (l.contains("defender")
                    && (l.contains("malware") || l.contains("virus") || l.contains("threat")))
                || (l.contains("defender")
                    && l.contains("scan")
                    && (l.contains("result") || l.contains("history") || l.contains("found")))
        }),
        ("permissions", |l| {
            l.contains("permission") || l.contains("access control") || l.contains("get-acl")
        }),
        ("login_history", |l| {
            l.contains("login history")
                || l.contains("logon history")
                || l.contains("event id 4624")
        }),
        ("registry_audit", |l| {
            l.contains("registry audit")
                || l.contains("persistence")
                || l.contains("ifeo")
                || l.contains("reg query")
        }),
        ("share_access", |l| {
            l.contains("share access")
                || l.contains("unc path")
                || l.contains("smbshare")
                || l.contains("net share")
                || (l.contains("network drive") && !l.contains("network drives"))
                || l.contains("mapped drive")
                || l.contains("shared folder")
        }),
        ("thermal", |l| {
            l.contains("thermal")
                || l.contains("throttling")
                || l.contains("overheat")
                || l.contains("too hot")
                || l.contains("running hot")
                || l.contains("cpu temp")
                || l.contains("cpu temperature")
                || l.contains("temp sensor")
                || l.contains("fan loud")
                || l.contains("fan noise")
                || l.contains("fan spinning")
                || l.contains("fan running")
                || l.contains("fans running")
                || l.contains("fans spinning")
                || l.contains("loud fan")
                || l.contains("fan always on")
                || (l.contains("fan") && l.contains("always on"))
                || l.contains("fan at 100")
                || l.contains("fan constantly")
                || l.contains("laptop hot")
                || l.contains("pc getting hot")
                || l.contains("getting hot")
                || l.contains("check temps")
                || (l.contains("temperature")
                    && (l.contains("cpu")
                        || l.contains("gpu")
                        || l.contains("system")
                        || l.contains("sensor")
                        || l.contains("check")))
        }),
        ("overclocker", |l| {
            l.contains("overclocker")
                || l.contains("gpu clock")
                || l.contains("nvidia stats")
                || l.contains("silicon health")
                || l.contains("mhz")
                || (l.contains("gpu")
                    && (l.contains("usage")
                        || l.contains("utilization")
                        || l.contains("performance")))
        }),
        ("activation", |l| {
            l.contains("activation")
                || l.contains("slmgr")
                || l.contains("license status")
                || l.contains("licensed")
                || l.contains("unlicensed")
                || (l.contains("activate") && l.contains("window"))
                || l.contains("not activated")
                || l.contains("product key")
                || l.contains("license expired")
                || l.contains("is windows genuine")
                || (l.contains("windows") && l.contains("activated"))
        }),
        ("patch_history", |l| {
            l.contains("patch history")
                || l.contains("hotfix")
                || l.contains("kb history")
                || l.contains("security patch")
                || (l.contains("update") && l.contains("applied"))
        }),
        ("bluetooth", |l| {
            l.contains("bluetooth")
                || l.contains("pairing")
                || l.contains("paired device")
                || l.contains("paired devices")
                || l.contains("bthserv")
                || l.contains("bthavctpsvc")
                || l.contains("btagservice")
                || l.contains("bluetoothuserservice")
                || ((l.contains("headset") || l.contains("headphones"))
                    && (l.contains("disconnect")
                        || l.contains("pair")
                        || l.contains("reconnect")
                        || l.contains("bluetooth")))
        }),
        ("audio", |l| {
            l.contains("no sound")
                || l.contains("audio service")
                || l.contains("windows audio")
                || l.contains("speaker")
                || l.contains("speakers")
                || l.contains("microphone")
                || l.contains(" mic ")
                || l.starts_with("mic ")
                || l.contains("mic not")
                || l.contains("headset")
                || l.contains("headphones")
                || l.contains("playback device")
                || l.contains("recording device")
                || l.contains("audio endpoint")
                || l.contains("audioendpointbuilder")
                || l.contains("can't hear")
                || l.contains("cannot hear")
                || l.contains("cant hear")
                || l.contains("no audio")
                || (((l.contains("audio") || l.contains("sound"))
                    && (l.contains("device")
                        || l.contains("driver")
                        || l.contains("service")
                        || l.contains("working")
                        || l.contains("broken")
                        || l.contains("input")
                        || l.contains("output")
                        || l.contains("crackling")
                        || l.contains("mute")
                        || l.contains("muted")
                        || l.contains("volume")
                        || l.contains("speaker")
                        || l.contains("microphone")))
                    && !l.contains("audio file")
                    && !l.contains("voice engine"))
        }),
        ("camera", |l| {
            l.contains("camera")
                || l.contains("webcam")
                || l.contains("web cam")
                || (l.contains("camera") && l.contains("permission"))
                || (l.contains("camera") && l.contains("privacy"))
        }),
        ("sign_in", |l| {
            l.contains("windows hello")
                || l.contains("sign in")
                || l.contains("cant sign in")
                || l.contains("can't sign in")
                || l.contains("logon failure")
                || l.contains("login screen stuck")
                || l.contains("stuck on login")
                || l.contains("login loop")
                || l.contains("can't login")
                || l.contains("cannot login")
                || l.contains("cant login")
                || l.contains("login failed")
                || l.contains("login failure")
                || (l.contains("login") && l.contains("not working"))
                || (l.contains("login") && l.contains("problem"))
                || (l.contains("login") && l.contains("status"))
                || l.contains("sign-in status")
                || l.contains("sign in status")
                || (l.contains("can't log in") && !l.contains("vpn") && !l.contains("ssh"))
                || (l.contains("pin") && (l.contains("broken") || l.contains("not working")))
                || l.contains("credential provider")
                || l.contains("biometric service")
                || l.contains("wbiosrvc")
        }),
        ("identity_auth", |l| {
            l.contains("web account manager")
                || l.contains("token broker")
                || l.contains("tokenbroker")
                || l.contains("aad broker")
                || l.contains("broker plugin")
                || l.contains("identity broker")
                || l.contains("microsoft 365 sign-in")
                || l.contains("microsoft 365 signin")
                || l.contains("office sign-in")
                || l.contains("office signin")
                || l.contains("workplace join")
                || l.contains("device registration")
                || l.contains("device registered")
                || l.contains("entra")
                || l.contains("azure ad")
                || l.contains("azuread")
                || l.contains("azure ad prt")
                || l.contains("azureadprt")
                || l.contains("wamdefaultset")
                || l.contains("single sign-on")
                || l.contains("organizational account")
                || l.contains("corporate account")
                || (l.contains("azure") && l.contains("registered"))
                || ((l.contains("outlook")
                    || l.contains("teams")
                    || l.contains("onedrive")
                    || l.contains("office")
                    || l.contains("microsoft 365"))
                    && (l.contains("sign in")
                        || l.contains("signin")
                        || l.contains("signed in")
                        || l.contains("signed out")
                        || l.contains("keeps asking")
                        || l.contains("keep asking")
                        || l.contains("authentication")
                        || l.contains("auth")
                        || l.contains("token")
                        || l.contains("work account")
                        || l.contains("school account")
                        || l.contains("account mismatch")))
        }),
        ("installer_health", |l| {
            l.contains("installer health")
                || l.contains("installer broken")
                || l.contains("msiexec")
                || l.contains("msi installer")
                || l.contains("windows installer")
                || l.contains("app installer")
                || l.contains("desktopappinstaller")
                || l.contains("microsoft store")
                || l.contains("winget broken")
                || (l.contains("unable to install")
                    && (l.contains("app") || l.contains("program") || l.contains("software")))
                || ((l.contains("install") || l.contains("installer"))
                    && (l.contains("fail")
                        || l.contains("failing")
                        || l.contains("broken")
                        || l.contains("stuck")
                        || l.contains("hanging")
                        || l.contains("error"))
                    && !l.contains("windows update"))
        }),
        ("onedrive", |l| {
            l.contains("onedrive")
                || l.contains("one drive")
                || l.contains("files on-demand")
                || l.contains("known folder backup")
                || l.contains("known folder move")
                || l.contains("kfm")
                || l.contains("sharepoint sync")
                || l.contains("sync root")
                || ((l.contains("desktop") || l.contains("documents") || l.contains("pictures"))
                    && l.contains("backup")
                    && (l.contains("onedrive") || l.contains("cloud") || l.contains("sync")))
                || ((l.contains("desktop") || l.contains("documents") || l.contains("pictures"))
                    && l.contains("sync")
                    && (l.contains("onedrive") || l.contains("sharepoint") || l.contains("cloud")))
        }),
        ("browser_health", |l| {
            l.contains("browser health")
                || l.contains("webview2")
                || l.contains("default browser")
                || ((l.contains("browser")
                    || l.contains("chrome")
                    || l.contains("edge")
                    || l.contains("firefox"))
                    && (l.contains("slow")
                        || l.contains("sluggish")
                        || l.contains("lag")
                        || l.contains("crash")
                        || l.contains("crashing")
                        || l.contains("hang")
                        || l.contains("freeze")
                        || l.contains("frozen")
                        || l.contains("broken")
                        || l.contains("unresponsive")
                        || l.contains("not starting")
                        || l.contains("not loading")
                        || l.contains("extension")
                        || l.contains("extensions")
                        || l.contains("proxy")
                        || l.contains("policy")))
                || ((l.contains("links") || l.contains("link"))
                    && (l.contains("open wrong")
                        || l.contains("opens wrong")
                        || l.contains("wrong browser")
                        || l.contains("wrong app")))
        }),
        ("outlook", |l| {
            l.contains("outlook")
                || l.contains("ms outlook")
                || l.contains("microsoft outlook")
                || (l.contains("ost") && l.contains("mail"))
                || (l.contains("pst") && l.contains("mail"))
                || (l.contains("add-in") && l.contains("mail"))
        }),
        ("teams", |l| {
            (l.contains("teams")
                && !l.contains("nic team")
                && !l.contains("nic teaming")
                && !l.contains("link aggregation")
                && !l.contains("lbfo"))
                || l.contains("ms teams")
                || l.contains("microsoft teams")
        }),
        ("windows_backup", |l| {
            l.contains("file history")
                || l.contains("windows backup")
                || l.contains("wbadmin")
                || l.contains("system restore")
                || l.contains("restore point")
                || l.contains("known folder move")
                || l.contains("backed up")
                || (l.contains("backup")
                    && (l.contains("drive")
                        || l.contains("running")
                        || l.contains("health")
                        || l.contains("status")
                        || l.contains("failed")
                        || l.contains("enabled")
                        || l.contains("working")
                        || l.contains("set up")
                        || l.contains("configured")
                        || l.contains("schedule")))
        }),
        ("search_index", |l| {
            l.contains("search index")
                || l.contains("windows search")
                || l.contains("wsearch")
                || l.contains("indexer")
                || (l.contains("search") && l.contains("broken"))
                || (l.contains("search") && l.contains("not working"))
        }),
        ("display_config", |l| {
            l.contains("monitor")
                || l.contains("display")
                || l.contains("resolution")
                || l.contains("refresh rate")
                || l.contains("refresh hz")
                || l.contains("dpi")
                || l.contains("scaling")
                || l.contains("screen config")
                || l.contains("hdmi")
                || l.contains("displayport")
                || l.contains("how many screens")
                || l.contains("multi-monitor")
                || l.contains("second screen")
                || l.contains("external display")
        }),
        ("ntp", |l| {
            l.contains("ntp")
                || l.contains("time sync")
                || l.contains("clock sync")
                || l.contains("w32tm")
                || l.contains("clock drift")
                || l.contains("time server")
                || l.contains("time zone")
                || l.contains("timezone")
                || l.contains("wrong timezone")
                || l.contains("clock wrong")
                || l.contains("time wrong")
                || l.contains("system clock")
                || l.contains("system time")
                || (l.contains("time") && l.contains("drift"))
                || (l.contains("clock") && l.contains("off"))
                || (l.contains("time") && l.contains("accurate"))
                || (l.contains("time") && l.contains("correct"))
        }),
        ("cpu_power", |l| {
            l.contains("turbo boost")
                || l.contains("cpu frequency")
                || l.contains("cpu freq")
                || l.contains("cpu clock")
                || l.contains("cpu power")
                || l.contains("cpu speed")
                || l.contains("processor speed")
                || l.contains("processor frequency")
                || l.contains("power plan")
                || l.contains("cpu stuck")
                || l.contains("boost disabled")
                || (l.contains("boost") && l.contains("disabled"))
                || (l.contains("cpu") && l.contains("slow"))
                || (l.contains("processor") && l.contains("slow"))
                || (l.contains("cpu") && l.contains("underclocking"))
                || (l.contains("processor") && l.contains("running slow"))
                || l.contains("processor running at")
        }),
        ("credentials", |l| {
            l.contains("credential manager")
                || l.contains("credential store")
                || l.contains("saved password")
                || l.contains("stored credential")
                || l.contains("credential vault")
                || l.contains("cmdkey")
                || (l.contains("credential") && l.contains("list"))
                || (l.contains("windows") && l.contains("credential"))
                || (l.contains("credential")
                    && (l.contains("clear")
                        || l.contains("cached")
                        || l.contains("view")
                        || l.contains("delete")
                        || l.contains("remove")))
        }),
        ("tpm", |l| {
            l.contains("tpm")
                || l.contains("secure boot")
                || l.contains("trusted platform module")
                || l.contains("firmware security")
                || l.contains("uefi security")
                || l.contains("uefi mode")
                || l.contains("uefi enabled")
                || l.contains("legacy bios")
                || l.contains("uefi bios")
                || (l.contains("uefi")
                    && (l.contains("boot")
                        || l.contains("secure")
                        || l.contains("status")
                        || l.contains("check")))
        }),
        ("dhcp", |l| {
            l.contains("dhcp lease")
                || l.contains("lease expires")
                || l.contains("dhcp server")
                || l.contains("ip lease")
                || l.contains("lease renew")
                || (l.contains("dhcp")
                    && (l.contains("detail") || l.contains("info") || l.contains("check")))
        }),
        ("mtu", |l| {
            l.contains("mtu")
                || l.contains("path mtu")
                || l.contains("pmtu")
                || l.contains("frame size")
                || l.contains("fragmentation")
                || (l.contains("vpn") && l.contains("mtu"))
                || (l.contains("packet") && l.contains("size") && l.contains("max"))
        }),
        ("latency", |l| {
            l.contains("ping")
                || l.contains("latency")
                || l.contains("packet loss")
                || l.contains("rtt")
                || l.contains("round trip")
                || l.contains("network lag")
                || l.contains("jitter")
                || (l.contains("network") && l.contains("slow"))
                || (l.contains("internet") && l.contains("slow"))
        }),
        ("network_adapter", |l| {
            l.contains("nic settings")
                || l.contains("nic offload")
                || l.contains("adapter settings")
                || l.contains("jumbo frame")
                || l.contains("tcp offload")
                || l.contains("wake on lan")
                || l.contains("wake-on-lan")
                || l.contains("link speed")
                || l.contains("duplex mismatch")
                || l.contains("adapter error")
                || (l.contains("nic") && (l.contains("driver") || l.contains("error")))
        }),
        ("ipv6", |l| {
            l.contains("ipv6")
                || l.contains("slaac")
                || l.contains("dhcpv6")
                || l.contains("privacy extension")
                || l.contains("global unicast")
        }),
        ("tcp_params", |l| {
            l.contains("tcp autotuning")
                || l.contains("tcp congestion")
                || l.contains("congestion algorithm")
                || l.contains("tcp settings")
                || l.contains("tcp tuning")
                || l.contains("tcp chimney")
                || l.contains("ecn")
                || l.contains("receive window")
                || l.contains("tcp window")
                || (l.contains("tcp")
                    && (l.contains("slow")
                        || l.contains("throughput")
                        || l.contains("speed")
                        || l.contains("window size")))
        }),
        ("wlan_profiles", |l| {
            l.contains("saved wifi")
                || l.contains("wifi profile")
                || l.contains("wlan profile")
                || l.contains("wireless profile")
                || l.contains("saved network")
                || l.contains("netsh wlan")
                || (l.contains("wifi")
                    && (l.contains("security")
                        || l.contains("audit")
                        || l.contains("remember")
                        || l.contains("password")))
                || (l.contains("wireless") && l.contains("remember"))
        }),
        ("ipsec", |l| {
            l.contains("ipsec")
                || l.contains("security association")
                || l.contains("ike tunnel")
                || l.contains("ipsec tunnel")
                || l.contains("policy agent")
                || l.contains("xfrm")
        }),
        ("netbios", |l| {
            l.contains("netbios")
                || l.contains("nbtstat")
                || l.contains("wins server")
                || l.contains("nbns")
        }),
        ("nic_teaming", |l| {
            l.contains("nic team")
                || l.contains("lacp")
                || l.contains("link aggregation")
                || l.contains("lbfo")
                || l.contains("bonding")
                || (l.contains("bond")
                    && (l.contains("adapter") || l.contains("interface") || l.contains("nic")))
        }),
        ("snmp", |l| {
            l.contains("snmp")
                || l.contains("community string")
                || l.contains("community name")
                || l.contains("snmpd")
        }),
        ("port_test", |l| {
            l.contains("port test")
                || l.contains("test port")
                || l.contains("port check")
                || l.contains("can i reach")
                || l.contains("is port")
                || l.contains("port reachab")
                || (l.contains("port")
                    && (l.contains("open") || l.contains("blocked") || l.contains("reachable")))
        }),
        ("network_profile", |l| {
            l.contains("network profile")
                || l.contains("network location")
                || l.contains("network category")
                || l.contains("public network")
                || l.contains("private network")
        }),
        ("dns_lookup", |l| {
            l.contains("dns lookup")
                || l.contains("dns record")
                || l.contains("nslookup")
                || l.contains("resolve-dnsname")
                || l.contains("gethostaddresses")
                || l.contains("gethostentry")
                || l.contains("mx record")
                || l.contains("srv record")
                || l.contains("look up ")
                || l.contains(" dig ")
                || l.starts_with("host ")
                || (l.contains("ip address") && l.contains(" of "))
                || (l.contains("resolve") && (l.contains("hostname") || l.contains("domain")))
        }),
        ("pending_reboot", |l| {
            l.contains("pending reboot")
                || l.contains("pending restart")
                || l.contains("need to restart")
                || l.contains("reboot required")
                || (l.contains("reboot") && l.contains("pending"))
                || (l.contains("restart") && l.contains("pending"))
                || (l.contains("have to") && (l.contains("restart") || l.contains("reboot")))
                || (l.contains("do i need to") && (l.contains("restart") || l.contains("reboot")))
        }),
        ("disk_health", |l| {
            l.contains("disk health")
                || l.contains("drive health")
                || l.contains("smart status")
                || l.contains("smart data")
                || l.contains("bad sector")
                || l.contains("ssd health")
                || l.contains("nvme health")
                || l.contains("hard drive status")
                || (l.contains("drive") && l.contains("status") && !l.contains("backup"))
                || (l.contains("healthy")
                    && (l.contains("drive") || l.contains("disk") || l.contains("ssd")))
                || (l.contains("disk") && l.contains("fail"))
        }),
        ("battery", |l| {
            l.contains("battery")
                || l.contains("charge percentage")
                || l.contains("current charge")
                || l.contains("charge status")
        }),
        ("app_crashes", |l| {
            l.contains("application crash")
                || l.contains("application error")
                || l.contains("app hang")
                || l.contains("application hang")
                || l.contains("faulting application")
                || l.contains("faulting module")
                || l.contains("wer report")
                || l.contains("apps crashing")
                || l.contains("what crashed")
                || l.contains("which app crashed")
                || l.contains("what app crashed")
                || l.contains("app crash history")
                || l.contains("application crash log")
                || (l.contains("crash") && l.contains("program"))
                || (l.contains("applications") && l.contains("crashing"))
                || (l.contains("apps") && l.contains("crashing"))
                || (l.contains("crash")
                    && (l.contains("chrome")
                        || l.contains("edge")
                        || l.contains("firefox")
                        || l.contains("discord")
                        || l.contains("office")))
        }),
        ("recent_crashes", |l| {
            l.contains("crash")
                || l.contains("bsod")
                || l.contains("blue screen")
                || l.contains("unexpected restart")
                || l.contains("sudden restart")
                || l.contains("keep restarting")
                || l.contains("keeps restarting")
                || l.contains("restarts randomly")
                || l.contains("random restart")
                || l.contains("random reboot")
                || l.contains("kernel panic")
                || (l.contains("restart") && l.contains("itself"))
                || (l.contains("restart") && l.contains("by itself"))
                || (l.contains("why") && l.contains("restart"))
        }),
        ("scheduled_tasks", |l| {
            l.contains("scheduled task")
                || l.contains("task scheduler")
                || l.contains("scheduled job")
                || l.contains("runs automatically")
                || l.contains("running automatically")
                || l.contains("auto-run task")
                || l.contains("what runs on a timer")
                || l.contains("cron job")
                || l.contains("background task")
                || (l.contains("background") && l.contains("run"))
                || (l.contains("periodic") && l.contains("run"))
                || (l.contains("what") && l.contains("schedule"))
        }),
        ("ad_user", |l| {
            l.contains("ad user")
                || l.contains("domain user")
                || (l.contains("user") && l.contains("sid"))
        }),
        ("dns_lookup", |l| {
            (l.contains("dns") && (l.contains("lookup") || l.contains("srv") || l.contains("mx")))
                || l.contains("resolve-dnsname")
                || l.contains("gethostaddresses")
                || l.contains("gethostentry")
                || l.starts_with("host ")
                || (l.contains("ip address") && l.contains(" of "))
        }),
        ("hyperv", |l| {
            l.contains("hyper-v")
                || l.contains("hyperv")
                || l.contains("hyper v")
                || l.contains("virtual machine")
                || l.contains("running vms")
                || (l.contains("vm")
                    && !l.contains("nvme")
                    && (l.contains("running")
                        || l.contains("checkpoint")
                        || l.contains("snapshot")
                        || l.contains("switch")
                        || l.contains("ram")
                        || l.contains("memory")))
                || (l.contains("list") && l.contains("vm") && !l.contains("nvme"))
        }),
        ("ip_config", |l| {
            l.contains("ipconfig") && (l.contains("all") || l.contains("detail"))
        }),
        ("dev_conflicts", |l| {
            l.contains("dev conflict")
                || l.contains("toolchain conflict")
                || l.contains("duplicate path")
        }),
        ("storage_deep", |l| {
            (l.contains("where did my")
                && (l.contains("space") || l.contains("disk") || l.contains("storage")))
                || l.contains("what is taking up")
                || l.contains("what is using my disk")
                || l.contains("what is eating my disk")
                || l.contains("biggest folders")
                || l.contains("largest folders")
                || l.contains("largest directories")
                || l.contains("what is filling")
                || l.contains("storage breakdown")
                || l.contains("disk breakdown")
                || l.contains("find large files")
                || l.contains("find big files")
                || l.contains("storage analysis")
                || l.contains("disk analysis")
                || (l.contains("clean up")
                    && (l.contains("disk")
                        || l.contains("drive")
                        || l.contains("space")
                        || l.contains("storage")))
                || (l.contains("clean") && l.contains("c drive"))
                || (l.contains("analyze") && (l.contains("storage") || l.contains("disk")))
        }),
        ("storage", |l| {
            l.contains("disk space")
                || l.contains("storage")
                || l.contains("drive capacity")
                || l.contains("cache size")
                || l.contains("i/o pressure")
                || l.contains("disk usage")
                || ((l.contains("disk") || l.contains("drive")) && l.contains("full"))
                || l.contains("out of space")
                || l.contains("free space")
                || l.contains("how much space")
                || l.contains("space left")
                || l.contains("running out of space")
                || l.contains("how much disk")
                || l.contains("how full")
                || (l.contains("drive") && l.contains("usage"))
                || (l.contains("where") && l.contains("space"))
        }),
        ("storage_spaces", |l| {
            l.contains("storage spaces")
                || l.contains("storage space")
                || l.contains("storage pool")
                || l.contains("storage pools")
                || l.contains("virtual disk")
                || l.contains("virtual disks")
                || l.contains("windows raid")
                || l.contains("storage pool degraded")
                || l.contains("parity volume")
                || l.contains("disk pool")
                || l.contains("resiliency")
                || l.contains("mdadm")
                || l.contains("software raid")
                || l.contains("md array")
                || (l.contains("mirror") && l.contains("drive"))
                || (l.contains("storage") && l.contains("pool"))
        }),
        ("disk_benchmark", |l| {
            l.contains("benchmark")
                || l.contains("stress test")
                || l.contains("load test")
                || l.contains("intensity report")
                || l.contains("io intensity")
                || l.contains("disk intensity")
                || l.contains("thrash")
                || l.contains("latency report")
        }),
        ("log_check", |l| {
            l.contains("event log")
                || l.contains("recent errors")
                || l.contains("recent warnings")
                || l.contains("error log")
                || l.contains("event viewer")
                || l.contains("system log")
                || l.contains("windows log")
                || l.contains("application log")
                || l.contains("recent events")
                || l.contains("journal log")
                || l.contains("show me warnings")
                || (l.contains("log") && l.contains("recent") && l.contains("error"))
                || (l.contains("log") && l.contains("warning"))
                || (l.contains("what errors") && l.contains("log"))
        }),
        ("hardware", |l| {
            l.contains("cpu model")
                || l.contains("ram size")
                || l.contains("hardware spec")
                || (l.contains("what hardware") && l.contains("have"))
                || l.contains("hardware info")
                || l.contains("hardware inventory")
                || l.contains("system information")
                || l.contains("system info")
                || l.contains("system specs")
                || l.contains("computer spec")
                || l.contains("graphics card")
                || l.contains("video card")
                || l.contains("display adapter")
                || l.contains("bios version")
                || l.contains("motherboard")
                || l.contains("what gpu")
                || l.contains("what cpu")
                || (l.contains("hardware") && (l.contains("dna") || l.contains("inventory")))
        }),
        ("health_report", |l| {
            l.contains("health report") || l.contains("system health")
        }),
        ("resource_load", |l| {
            l.contains("resource load")
                || l.contains("cpu load")
                || l.contains("ram %")
                || l.contains("cpu %")
                || l.contains("performance")
                || l.contains("memory pressure")
                || l.contains("memory load")
                || l.contains("slow")
                || l.contains("lag")
                || l.contains("sluggish")
                || l.contains("hang")
                || l.contains("unresponsive")
                || l.contains("frozen")
                || l.contains("freezing up")
                || l.contains("computer freeze")
                || l.contains("utilization")
                || l.contains("high cpu")
                || l.contains("high ram")
                || l.contains("current load")
                || (l.contains("resource") && l.contains("usage"))
                || (l.contains("why") && l.contains("slow"))
                || (l.contains("why") && l.contains("laggy"))
        }),
        ("processes", |l| {
            l.contains("process")
                || l.contains("task manager")
                || l.contains("what is running")
                || l.contains("using my ram")
                || l.contains("using my cpu")
                || l.contains("using the cpu")
                || l.contains("hitting the disk")
                || l.contains("disk thrasher")
                || l.contains("cpu hog")
                || l.contains("memory hog")
                || l.contains("ram hog")
                || l.contains("hogging cpu")
                || l.contains("hogging ram")
                || l.contains("hogging memory")
                || l.contains("eating up cpu")
                || l.contains("eating up memory")
                || l.contains("eating my cpu")
                || l.contains("eating my memory")
                || (l.contains("hogging")
                    && (l.contains("cpu") || l.contains("ram") || l.contains("memory")))
                || (l.contains("eating up")
                    && (l.contains("cpu") || l.contains("ram") || l.contains("memory")))
        }),
        ("services", |l| {
            l.contains("service") || l.contains("daemon") || l.contains("windows service")
        }),
        ("ports", |l| {
            l.contains("listening port")
                || l.contains("open port")
                || l.contains("what is on port")
                || l.contains("port 3000")
                || l.contains("what is listening")
                || l.contains("what's listening")
                || l.contains("exposed port")
                || l.contains("listening on port")
                || (l.contains("listening") && l.contains("port"))
        }),
        ("traceroute", |l| {
            l.contains("traceroute")
                || l.contains("tracert")
                || l.contains("trace route")
                || l.contains("trace the path")
                || l.contains("network path")
                || l.contains("how many hops")
                || (l.contains("trace") && l.contains("hop"))
        }),
        ("dns_cache", |l| {
            l.contains("dns cache")
                || l.contains("cached dns")
                || l.contains("displaydns")
                || (l.contains("dns") && l.contains("cached"))
        }),
        ("arp", |l| {
            l.contains("arp table")
                || l.contains("arp cache")
                || l.contains("mac address")
                || l.contains("ip to mac")
                || l.contains("arp -")
        }),
        ("route_table", |l| {
            l.contains("route table")
                || l.contains("routing table")
                || l.contains("route print")
                || l.contains("network route")
                || l.contains("next hop")
        }),
        ("connectivity", |l| {
            l.contains("internet")
                || l.contains("am i connected")
                || l.contains("ping google")
                || l.contains("internet access")
                || l.contains("no internet")
                || l.contains("network connectivity")
                || l.contains("connectivity check")
                || l.contains("check connectivity")
                || l.contains("can't browse")
                || l.contains("cannot browse")
                || l.contains("web browsing")
                || l.contains("browser not loading")
                || l.contains("pages not loading")
                || l.contains("websites not loading")
                || l.contains("no network")
                || (l.contains("network") && l.contains("down"))
                || (l.contains("can't") && l.contains("connect to internet"))
                || (l.contains("cannot") && l.contains("connect to internet"))
                || (l.contains("network") && l.contains("not working"))
                || (l.contains("internet") && l.contains("not working"))
        }),
        ("wifi", |l| {
            l.contains("wi-fi")
                || l.contains("wifi")
                || l.contains("wireless")
                || l.contains("ssid")
                || l.contains("signal strength")
        }),
        ("connections", |l| {
            l.contains("tcp connection")
                || l.contains("active connection")
                || l.contains("established connection")
                || l.contains("socket")
                || l.contains("netstat")
                || l.contains("outbound connection")
                || l.contains("inbound connection")
                || l.contains("remote connection")
                || l.contains("connection list")
                || (l.contains("connection") && l.contains("active"))
                || (l.contains("connection") && l.contains("open"))
                || (l.contains("what") && l.contains("connecting"))
                || (l.contains("which") && l.contains("connecting"))
        }),
        ("vpn", |l| {
            l.contains("vpn")
                || l.contains("virtual private network")
                || l.contains("wireguard")
                || l.contains("anyconnect")
                || l.contains("globalprotect")
                || l.contains("pulse secure")
                || l.contains("split tunnel")
                || l.contains("vpn adapter")
        }),
        ("proxy", |l| {
            l.contains("proxy setting") || l.contains("system proxy") || l.contains("winhttp proxy")
        }),
        ("firewall_rules", |l| {
            (l.contains("firewall")
                && (l.contains("rule") || l.contains("inbound") || l.contains("outbound")))
                || l.contains("firewall rule")
        }),
        ("lan_discovery", |l| {
            l.contains("upnp")
                || l.contains("ssdp")
                || l.contains("mdns")
                || l.contains("bonjour")
                || l.contains("llmnr")
                || l.contains("network neighborhood")
                || l.contains("device discovery")
                || l.contains("local discovery")
                || l.contains("discover local devices")
                || l.contains("discover devices")
                || l.contains("browse computers")
                || (l.contains("local network")
                    && (l.contains("discover")
                        || l.contains("discovery")
                        || l.contains("neighborhood")
                        || l.contains("device")
                        || l.contains("devices")
                        || l.contains("aware of")))
                || ((l.contains("netbios") || l.contains("smb visibility"))
                    && !l.contains("active directory"))
                || ((l.contains("nas")
                    || l.contains("printer")
                    || l.contains("device")
                    || l.contains("computer")
                    || l.contains("pc"))
                    && ((l.contains("can't") && l.contains("see"))
                        || (l.contains("cannot") && l.contains("see"))
                        || (l.contains("cant") && l.contains("see"))
                        || l.contains("can't see")
                        || l.contains("cannot see")
                        || l.contains("cant see")
                        || l.contains("not visible")
                        || l.contains("not showing up")
                        || l.contains("not show up")
                        || l.contains("discover"))
                    && (l.contains("network")
                        || l.contains("lan")
                        || l.contains("local")
                        || l.contains("neighborhood")))
        }),
        ("network", |l| {
            l.contains("network adapter")
                || l.contains("ip address")
                || l.contains("ipconfig")
                || l.contains("gateway")
                || l.contains("subnet")
        }),
        ("env_doctor", |l| {
            l.contains("env doctor")
                || l.contains("environment doctor")
                || l.contains("package manager")
                || l.contains("path drift")
        }),
        ("os_config", |l| {
            l.contains("power plan")
                || l.contains("uptime")
                || l.contains("boot time")
                || l.contains("last boot")
        }),
        ("overclocker", |l| {
            l.contains("overclocker")
                || l.contains("gpu clock")
                || l.contains("gpu throttle")
                || l.contains("nvidia stats")
                || l.contains("silicon health")
                || l.contains("mhz")
                || ((l.contains("voltage") || l.contains("volts"))
                    && (l.contains("gpu")
                        || l.contains("cpu")
                        || l.contains("nvidia")
                        || l.contains("silicon")))
                || (l.contains("gpu")
                    && (l.contains("throttle")
                        || l.contains("bottleneck")
                        || l.contains("overheating")
                        || l.contains("usage")
                        || l.contains("utilization")
                        || l.contains("performance")))
        }),
        ("path", |l| {
            l.contains("path entries") || l.contains("raw path")
        }),
        ("toolchains", |l| {
            l.contains("developer tools")
                || l.contains("toolchains")
                || (l.contains("installed") && l.contains("version"))
        }),
        ("docker", |l| {
            l.contains("docker") || l.contains("container") || l.contains("running container")
        }),
        ("docker_filesystems", |l| {
            (l.contains("docker")
                || l.contains("container")
                || l.contains("compose")
                || l.contains("volume")
                || l.contains("bind mount"))
                && (l.contains("mount")
                    || l.contains("volume")
                    || l.contains("bind")
                    || l.contains("filesystem")
                    || l.contains("storage")
                    || l.contains("path")
                    || l.contains("missing"))
        }),
        ("wsl", |l| {
            l.contains("wsl")
                || l.contains("windows subsystem")
                || (l.contains("subsystem") && l.contains("linux"))
        }),
        ("wsl_filesystems", |l| {
            (l.contains("wsl")
                || l.contains("windows subsystem")
                || l.contains("linux distro")
                || (l.contains("subsystem") && l.contains("linux")))
                && (l.contains("mount")
                    || l.contains("filesystem")
                    || l.contains("storage")
                    || l.contains("disk")
                    || l.contains("vhdx")
                    || l.contains("path bridge")
                    || l.contains("/mnt/c")
                    || l.contains("wsl df")
                    || l.contains("wsl du")
                    || l.contains("du -sh /mnt/c"))
        }),
        ("ssh", |l| {
            l.contains("ssh")
                || l.contains("sshd")
                || l.contains("known_hosts")
                || l.contains("authorized_keys")
        }),
        ("git_config", |l| {
            (l.contains("git config") || l.contains("git global") || l.contains("git aliases"))
                && !l.contains("github")
        }),
        ("installed_software", |l| {
            l.contains("installed software")
                || l.contains("installed program")
                || l.contains("installed app")
                || l.contains("what is installed")
                || l.contains("what's installed")
                || l.contains("winget list")
                || l.contains("list programs")
                || l.contains("list applications")
                || l.contains("list apps")
                || l.contains("show applications")
                || l.contains("show programs")
                || (l.contains("list") && l.contains("application"))
                || (l.contains("show") && l.contains("application"))
        }),
        ("env", |l| {
            (l.contains("environment variable") || l.contains("env var") || l.contains("env vars"))
                && !l.contains("env doctor")
        }),
        ("hosts_file", |l| {
            l.contains("hosts file") || l.contains("/etc/hosts") || l.contains("hosts entry")
        }),
        ("databases", |l| {
            l.contains("postgres")
                || l.contains("mysql")
                || l.contains("mariadb")
                || l.contains("mongodb")
                || l.contains("redis")
                || l.contains("sqlite")
                || l.contains("sql server")
                || l.contains("elasticsearch")
                || (l.contains("database") && (l.contains("running") || l.contains("service")))
        }),
        ("user_accounts", |l| {
            l.contains("local user")
                || l.contains("user account")
                || l.contains("who is logged")
                || l.contains("who am i")
                || l.contains("logged in as")
                || l.contains("admin group")
                || l.contains("local admin")
                || l.contains("active sessions")
                || l.contains("running as admin")
                || l.contains("who has admin rights")
                || l.contains("list all users")
                || l.contains("list users")
                || l.contains("what accounts")
                || (l.contains("accounts") && l.contains("admin"))
        }),
        ("audit_policy", |l| {
            l.contains("audit policy")
                || l.contains("auditpol")
                || l.contains("what is being logged")
                || l.contains("security audit")
                || l.contains("event auditing")
                || l.contains("login event")
                || l.contains("logon event")
                || (l.contains("audit") && l.contains("event"))
                || (l.contains("what") && l.contains("being audited"))
                || (l.contains("audit") && l.contains("enable"))
        }),
        ("shares", |l| {
            l.contains("smb share")
                || l.contains("network share")
                || l.contains("shared folder")
                || l.contains("mapped drive")
                || l.contains("file sharing")
                || l.contains("what is shared")
                || l.contains("what am i sharing")
                || l.contains("smb1")
                || l.contains("nfs export")
                || (l.contains("folder") && l.contains("shared"))
                || (l.contains("sharing") && l.contains("network"))
        }),
        ("dns_servers", |l| {
            (l.contains("dns server")
                || l.contains("dns resolver")
                || l.contains("nameserver")
                || l.contains("which dns")
                || l.contains("dns over https")
                || l.contains("configured dns"))
                && !l.contains("dns cache")
        }),
        ("bitlocker", |l| {
            l.contains("bitlocker")
                || (l.contains("drive") && l.contains("encrypt"))
                || (l.contains("disk") && l.contains("encrypt"))
                || (l.contains("ssd") && l.contains("encrypt"))
                || (l.contains("volume") && l.contains("encrypt"))
                || (l.contains("machine") && l.contains("encrypt"))
                || l.contains("encryption status")
                || l.contains("full disk encryption")
                || l.contains("drive encryption")
        }),
        ("rdp", |l| {
            l.contains("rdp")
                || l.contains("remote desktop")
                || (l.contains("remote") && l.contains("access") && !l.contains("git"))
        }),
        ("shadow_copies", |l| {
            l.contains("shadow copy")
                || l.contains("shadow copies")
                || l.contains("vss")
                || l.contains("snapshot")
                || l.contains("restore point")
        }),
        ("pagefile", |l| {
            l.contains("pagefile")
                || l.contains("page file")
                || l.contains("virtual memory")
                || l.contains("swap file")
                || l.contains("swap space")
                || l.contains("memory swapping")
                || l.contains("paging file")
        }),
        ("windows_features", |l| {
            (l.contains("window") && l.contains("feature"))
                || l.contains("optional feature")
                || l.contains("iis")
                || l.contains("hyper-v")
                || (l.contains("feature") && (l.contains("install") || l.contains("enabled")))
        }),
        ("printers", |l| {
            l.contains("printer")
                || l.contains("print queue")
                || l.contains("get-printer")
                || l.contains("printing")
                || l.contains("can't print")
                || l.contains("cannot print")
                || l.contains("print job")
                || l.contains("print driver")
                || l.contains("default printer")
                || (l.contains("print") && l.contains("not working"))
                || (l.contains("print") && l.contains("stuck"))
                || (l.contains("print") && l.contains("offline"))
        }),
        ("winrm", |l| {
            l.contains("winrm")
                || l.contains("psremoting")
                || (l.contains("remote") && l.contains("management") && !l.contains("rdp"))
        }),
        ("network_stats", |l| {
            (l.contains("network") && l.contains("stat"))
                || (l.contains("adapter")
                    && l.contains("stat")
                    && !l.contains("wlan")
                    && !l.contains("wireless"))
                || l.contains("throughput")
                || l.contains("packet loss")
                || l.contains("dropped packet")
                || (l.contains("network") && l.contains("usage"))
                || (l.contains("data") && l.contains("transferred"))
                || (l.contains("bytes") && l.contains("transferred"))
                || l.contains("network traffic")
                || (l.contains("packet") && l.contains("error"))
        }),
        ("startup_items", |l| {
            l.contains("startup")
                || l.contains("boot program")
                || l.contains("autorun")
                || l.contains("run at boot")
                || l.contains("startup program")
                || l.contains("startup app")
                || l.contains("startup list")
                || l.contains("startup item")
                || l.contains("starts with windows")
                || l.contains("start with windows")
                || l.contains("launch at startup")
                || l.contains("launch on startup")
                || l.contains("open at startup")
                || l.contains("open on boot")
                || l.contains("runs on boot")
                || l.contains("run at login")
                || l.contains("msconfig")
                || l.contains("login item")
                || (l.contains("disable") && l.contains("startup"))
                || (l.contains("what") && l.contains("start") && l.contains("boot"))
                || l.contains("autostart")
                || (l.contains("load") && l.contains("boot"))
                || (l.contains("load") && l.contains("startup") && !l.contains("reload"))
        }),
        ("udp_ports", |l| {
            l.contains("udp port")
                || l.contains("udp listener")
                || (l.contains("udp")
                    && (l.contains("listening")
                        || l.contains("service")
                        || l.contains("connection")
                        || l.contains("open")))
        }),
        ("gpo", |l| {
            l.contains("gpo")
                || l.contains("group polic")
                || l.contains("gpresult")
                || l.contains("applied policy")
                || l.contains("active policies")
                || l.contains("what policies")
                || l.contains("policy objects")
                || l.contains("policy applied")
                || (l.contains("policies") && l.contains("applied"))
                || (l.contains("policies") && l.contains("effect"))
        }),
        ("certificates", |l| {
            l.contains("cert")
                || l.contains("ssl")
                || l.contains("client cert")
                || l.contains("expiring cert")
                || l.contains("tls certificate")
                || l.contains("thumbprint")
                || l.contains("x509")
                || l.contains("x.509")
                || l.contains(".pfx")
                || l.contains(".p12")
                || l.contains(".pem")
                || l.contains("pkcs")
                || l.contains("trust store")
                || l.contains("certificate store")
                || l.contains("certificate expir")
                || l.contains("untrusted cert")
                || (l.contains("tls")
                    && (l.contains("check") || l.contains("status") || l.contains("valid")))
        }),
        ("integrity", |l| {
            l.contains("integrity")
                || l.contains("sfc")
                || l.contains("dism")
                || l.contains("corrupt")
                || l.contains("system file")
                || (l.contains("windows") && l.contains("damaged"))
                || (l.contains("check") && l.contains("system") && l.contains("file"))
        }),
        ("domain", |l| {
            l.contains("domain") || l.contains("workgroup") || l.contains("active directory")
        }),
        ("device_health", |l| {
            l.contains("device health")
                || l.contains("hardware error")
                || l.contains("yellow bang")
                || l.contains("malfunctioning")
                || l.contains("device manager")
                || l.contains("unknown device")
                || l.contains("code 43")
                || l.contains("code 10")
                || l.contains("code 28")
                || l.contains("hardware failing")
                || (l.contains("device") && l.contains("error code"))
                || (l.contains("device") && l.contains("not recognized"))
                || (l.contains("device") && l.contains("broken"))
                || (l.contains("device") && l.contains("not working"))
                || (l.contains("device") && l.contains("stopped working"))
                || (l.contains("hardware") && l.contains("broken"))
        }),
        ("drivers", |l| {
            l.contains("driver") || l.contains("system driver")
        }),
        ("peripherals", |l| {
            l.contains("peripheral")
                || l.contains("usb")
                || l.contains("keyboard")
                || l.contains("mouse")
                || l.contains("monitor")
        }),
        ("sessions", |l| {
            l.contains("session")
                || l.contains("who is logged")
                || l.contains("active login")
                || l.contains("who is on")
                || l.contains("connected users")
                || l.contains("logged on users")
                || l.contains("logged in users")
                || l.contains("query session")
                || l.contains("qwinsta")
                || (l.contains("who") && l.contains("logged"))
                || (l.contains("who")
                    && l.contains("using")
                    && (l.contains("computer") || l.contains("machine")))
        }),
        ("hardware", |l| {
            l.contains("virtualization")
                || l.contains("hypervisor")
                || l.contains("vt-x")
                || l.contains("slat")
                || l.contains("cpu model")
                || l.contains("ram size")
                || l.contains("hardware spec")
                || l.contains("motherboard")
                || l.contains("bios version")
                || l.contains("graphics card")
                || l.contains("video card")
                || l.contains("system information")
                || l.contains("system info")
                || l.contains("system specs")
                || l.contains("computer spec")
                || l.contains("display adapter")
                || (l.contains("what") && l.contains("processor"))
                || (l.contains("what") && l.contains("cpu") && !l.contains("using"))
                || (l.contains("how much") && l.contains("ram"))
                || (l.contains("gpu") && (l.contains("what") || l.contains("show")))
        }),
        ("ipv6", |l| {
            l.contains("ipv6")
                || l.contains("slaac")
                || l.contains("dhcpv6")
                || l.contains("ipv6 address")
                || l.contains("privacy extension")
                || l.contains("link-local address")
        }),
        ("domain_health", |l| {
            l.contains("domain health")
                || l.contains("dc connectivity")
                || l.contains("dc reachab")
                || l.contains("reach dc")
                || l.contains("domain controller")
                || l.contains("kerberos health")
                || l.contains("kerberos connectivity")
                || l.contains("nltest")
                || l.contains("dsgetdc")
                || l.contains("ldap port")
                || l.contains("ldap error")
                || l.contains("ad connectivity")
                || l.contains("gpo refresh")
                || (l.contains("active directory") && l.contains("connect"))
                || (l.contains("active directory") && l.contains("reach"))
                || (l.contains("active directory") && l.contains("health"))
                || (l.contains("active directory") && l.contains("working"))
                || (l.contains("active directory") && l.contains("accessible"))
                || (l.contains("domain controller") && l.contains("online"))
                || (l.contains("can reach") && l.contains("domain"))
                || (l.contains("kerberos") && l.contains("issue"))
                || (l.contains("kerberos") && l.contains("fail"))
        }),
        ("service_dependencies", |l| {
            l.contains("service depend")
                || l.contains("services depend")
                || l.contains("depends on")
                || l.contains("what depends on")
                || l.contains("which services depend")
                || l.contains("restart cascade")
                || l.contains("service graph")
                || l.contains("svc dep")
                || (l.contains("service") && l.contains("dependenc"))
                || (l.contains("service") && l.contains("required by"))
                || (l.contains("service") && l.contains("needed by"))
                || l.contains("service prerequisite")
                || (l.contains("service") && l.contains("requirement"))
                || (l.contains("service") && l.contains("relationship"))
        }),
        ("wmi_health", |l| {
            l.contains("wmi health")
                || l.contains("wmi corrupt")
                || l.contains("wmi repository")
                || l.contains("wmi broken")
                || l.contains("wmi not working")
                || l.contains("wmi query fail")
                || l.contains("wmi error")
                || l.contains("winmgmt")
                || (l.contains("wmi")
                    && (l.contains("health")
                        || l.contains("corrupt")
                        || l.contains("reposit")
                        || l.contains("broken")
                        || l.contains("repair")
                        || l.contains("status")
                        || l.contains("reset")))
        }),
        ("local_security_policy", |l| {
            l.contains("password policy")
                || l.contains("account lockout")
                || l.contains("lockout policy")
                || l.contains("lockout threshold")
                || l.contains("lm compatibility")
                || l.contains("lmcompatibilitylevel")
                || l.contains("ntlm policy")
                || l.contains("ntlm level")
                || l.contains("ntlm authentication level")
                || l.contains("local security policy")
                || l.contains("uac policy")
                || l.contains("uac disabled")
                || l.contains("uac level")
                || l.contains("uac prompt")
                || l.contains("user account control")
                || l.contains("needs elevation")
                || l.contains("run as administrator")
                || l.contains("administrator permission")
                || l.contains("account policy")
                || l.contains("net accounts")
                || l.contains("lockout")
                || (l.contains("uac")
                    && (l.contains("off")
                        || l.contains("status")
                        || l.contains("check")
                        || l.contains("on")))
        }),
        ("usb_history", |l| {
            l.contains("usb history")
                || l.contains("usb forensic")
                || l.contains("usb devices connected")
                || l.contains("usb devices ever")
                || l.contains("usb drives ever")
                || l.contains("what usb")
                || l.contains("usbstor")
                || l.contains("usb registry")
                || (l.contains("usb") && l.contains("ever connected"))
                || (l.contains("usb") && l.contains("audit"))
                || (l.contains("usb") && l.contains("forensic"))
                || (l.contains("usb") && l.contains("history"))
                || (l.contains("usb") && l.contains("registry"))
                || (l.contains("usb") && l.contains("plugged"))
                || (l.contains("usb") && l.contains("were connected"))
                || (l.contains("usb") && l.contains("have been connected"))
                || (l.contains("usb") && l.contains("has been connected"))
        }),
        ("print_spooler", |l| {
            l.contains("print spooler")
                || l.contains("printnightmare")
                || l.contains("print nightmar")
                || l.contains("spooler service")
                || l.contains("print security")
                || l.contains("cve-2021-34527")
                || l.contains("cve-2021-1675")
                || l.contains("printer security")
                || l.contains("printer service")
                || l.contains("point and print")
                || l.contains("rpcauthnlevel")
                || l.contains("print service")
                || (l.contains("printer") && l.contains("spooler"))
                || (l.contains("print") && l.contains("vulnerab"))
                || (l.contains("spooler") && l.contains("status"))
                || (l.contains("spooler") && l.contains("running"))
                || (l.contains("spooler") && l.contains("hardening"))
        }),
        ("repo_doctor", |l| {
            l.contains("repo doctor")
                || l.contains("repository doctor")
                || l.contains("workspace health")
                || l.contains("repo health")
                || l.contains("git status")
                || l.contains("uncommitted changes")
        }),
        ("desktop", |l| {
            l.contains("desktop")
                && (l.contains("show")
                    || l.contains("list")
                    || l.contains("what is in")
                    || l.contains("what's in")
                    || l.contains("folder")
                    || l.contains("contents"))
        }),
        ("downloads", |l| {
            l.contains("downloads")
                && (l.contains("show")
                    || l.contains("list")
                    || l.contains("what is in")
                    || l.contains("what's in")
                    || l.contains("folder")
                    || l.contains("contents"))
        }),
    ];

    for (topic, check) in detectors {
        if check(&lower) && !topics.contains(topic) {
            topics.push(topic);
        }
    }

    if topics.contains(&"docker_filesystems") {
        topics.retain(|topic| *topic != "docker");
        topics.retain(|topic| *topic != "storage");
    }
    if topics.contains(&"wsl_filesystems") {
        topics.retain(|topic| *topic != "wsl");
        topics.retain(|topic| *topic != "storage");
    }
    if topics.contains(&"lan_discovery") {
        topics.retain(|topic| *topic != "network");
    }
    if topics.contains(&"dns_lookup") {
        topics.retain(|topic| *topic != "network");
    }
    if topics.contains(&"identity_auth") {
        topics.retain(|topic| *topic != "sign_in");
        topics.retain(|topic| *topic != "onedrive");
        topics.retain(|topic| *topic != "outlook");
        topics.retain(|topic| *topic != "teams");
        topics.retain(|topic| *topic != "browser_health");
    }
    if topics.contains(&"event_query") {
        topics.retain(|topic| *topic != "log_check");
    }
    if topics.contains(&"browser_health") {
        topics.retain(|topic| *topic != "proxy");
    }
    if topics.contains(&"audio") {
        topics.retain(|topic| *topic != "peripherals");
    }
    if topics.contains(&"bluetooth") {
        topics.retain(|topic| *topic != "peripherals");
    }

    topics
}

pub(crate) fn preferred_maintainer_workflow(user_input: &str) -> Option<&'static str> {
    let lower = user_input.to_ascii_lowercase();
    let asks_cleanup = contains_any(
        &lower,
        &[
            "run my cleanup",
            "run the cleanup",
            "run cleanup",
            "deep clean",
            "prune dist",
            "clean.ps1",
            "cleanup script",
            "cleanup workflow",
            "clean up scripts",
        ],
    );
    let asks_package = contains_any(
        &lower,
        &[
            "rebuild local portable",
            "rebuild the portable",
            "run the local build",
            "run the portable",
            "package-windows.ps1",
            "package windows",
            "build installer",
            "overwrite the portable",
            "refresh the portable",
            "update path",
            "update path with the portable",
        ],
    );
    let asks_release = contains_any(
        &lower,
        &[
            "run the release flow",
            "regular workflow",
            "cut the release",
            "ship it",
            "release.ps1",
            "bump to ",
            "tag it",
            "full tag and everything",
            "publish crates",
        ],
    );

    if asks_cleanup {
        Some("clean")
    } else if asks_package {
        Some("package_windows")
    } else if asks_release {
        Some("release")
    } else {
        None
    }
}

pub fn mentions_symbol_search(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "find where",
            "who calls",
            "who uses",
            "where is",
            "is defined",
            "is used",
            "find definition",
            "find references",
            "go to definition",
        ],
    ) && contains_any(
        &lower,
        &[
            "function", "struct", "variable", "symbol", "method", "type", "trait", "module",
        ],
    )
}

pub fn mentions_commit_intent(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "git commit",
            "commit my",
            "commit the",
            "commit changes",
            "save my progress to git",
        ],
    )
}

pub fn preferred_workspace_workflow(user_input: &str) -> Option<&'static str> {
    let lower = user_input.to_ascii_lowercase();
    let asks_project_scope = contains_any(
        &lower,
        &[
            "this repo",
            "this repository",
            "this project",
            "current project",
            "current repo",
            "workspace",
            "in this folder",
            "here",
        ],
    );
    let asks_build = contains_any(
        &lower,
        &[
            "run the build",
            "build this project",
            "build this repo",
            "run build",
            "compile this project",
            "cargo build",
            "npm run build",
            "pnpm run build",
            "yarn build",
            "go build",
            "gradlew build",
        ],
    );
    let asks_test = contains_any(
        &lower,
        &[
            "run the tests",
            "run tests",
            "test this project",
            "test this repo",
            "run the test suite",
            "cargo test",
            "npm test",
            "pnpm test",
            "yarn test",
            "pytest",
            "go test",
            "gradlew test",
        ],
    );
    let asks_lint = contains_any(
        &lower,
        &[
            "run lint",
            "lint this project",
            "lint this repo",
            "cargo clippy",
            "npm run lint",
            "pnpm run lint",
            "yarn lint",
        ],
    );
    let asks_fix = contains_any(
        &lower,
        &[
            "run fix",
            "fix formatting",
            "run formatter",
            "cargo fmt",
            "npm run fix",
            "pnpm run fix",
            "yarn fix",
        ],
    );
    let asks_script = {
        let is_make_file_op = lower.contains("make a folder")
            || lower.contains("make a directory")
            || lower.contains("make a file")
            || lower.contains("make a hello.txt")
            || lower.contains("make it")
            || lower.contains("make x");

        let has_script_keyword = contains_any(
            &lower,
            &[
                "npm run ",
                "pnpm run ",
                "yarn ",
                "bun run ",
                "make ",
                "just ",
                "task ",
                "scripts/",
                ".\\scripts\\",
                "./scripts/",
                ".ps1",
                ".sh",
                ".py",
                ".cmd",
                ".bat",
            ],
        );

        has_script_keyword && !is_make_file_op
    };

    if mentions_symbol_search(user_input) {
        Some("lsp_search")
    } else if mentions_commit_intent(user_input) {
        Some("commit_workflow")
    } else if asks_build
        && (asks_project_scope
            || !contains_any(&lower, &["release.ps1", "package-windows.ps1", "clean.ps1"]))
    {
        Some("build")
    } else if asks_test && asks_project_scope {
        Some("test")
    } else if asks_lint && asks_project_scope {
        Some("lint")
    } else if asks_fix && asks_project_scope {
        Some("fix")
    } else if asks_script && preferred_maintainer_workflow(user_input).is_none() {
        Some("script")
    } else if (asks_test || asks_lint || asks_fix)
        && preferred_maintainer_workflow(user_input).is_none()
    {
        Some(if asks_test {
            "test"
        } else if asks_lint {
            "lint"
        } else {
            "fix"
        })
    } else {
        None
    }
}

pub(crate) fn looks_like_mutation_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    [
        "fix ",
        "change ",
        "edit ",
        "modify ",
        "update ",
        "rename ",
        "refactor ",
        "patch ",
        "rewrite ",
        "implement ",
        "create a file",
        "create file",
        "add a file",
        "delete ",
        "remove ",
        "make the change",
        "mkdir ",
        "touch ",
        "create a folder",
        "create folder",
        "new folder",
        "new file",
        "write to",
        "save this",
        "commit ",
        "move-item",
        "remove-item",
        "copy-item",
        "rmdir",
        "mv ",
        "rm ",
        "cp ",
        "set-content",
        "add-content",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn is_sovereign_mutation(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let mentions_location = contains_any(
        &lower,
        &[
            "desktop",
            "documents",
            "downloads",
            "pictures",
            "images",
            "videos",
            "movies",
            "music",
            "audio",
            "temp",
            "cache",
            "config",
            "appdata",
        ],
    );
    let mentions_simple_creation = (lower.contains("make")
        || lower.contains("create")
        || lower.contains("add")
        || lower.contains("new")
        || lower.contains("mkdir")
        || lower.contains("generate"))
        && (lower.contains("folder")
            || lower.contains("directory")
            || lower.contains("project area")
            || lower.contains("file"));

    mentions_location && mentions_simple_creation
}

pub fn classify_query_intent(workflow_mode: WorkflowMode, user_input: &str) -> QueryIntent {
    let lower = user_input.to_lowercase();

    let mentions_runtime_trace = contains_any(
        &lower,
        &[
            "trace",
            "how does",
            "what are the main runtime subsystems",
            "how does a user message move",
            "separate normal assistant output",
            "session reset behavior",
            "file references",
            "event types",
            "channels",
        ],
    );
    let anti_guess = contains_any(&lower, &["do not guess", "if you are unsure"]);
    let capability_mode = mentions_capability_question(&lower);
    let capability_needs_repo =
        capability_mode && capability_question_requires_repo_inspection(&lower);
    let is_coding_workflow =
        workflow_mode == WorkflowMode::Auto || workflow_mode == WorkflowMode::Code;

    let has_authoritative_hardware_noun = lower.split_whitespace().any(|w| {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric());
        matches!(
            w,
            "gpu"
                | "ram"
                | "cpu"
                | "vram"
                | "nvidia"
                | "silicon"
                | "vitals"
                | "throttle"
                | "overclocker"
                | "thermal"
        )
    });

    let host_inspection_allowed = if is_coding_workflow && code_kw_ac().find(&lower).is_some() {
        // High-barrier: if we are clearly in a code task, only allow diagnostic
        // if they use an authoritative hardware noun.
        has_authoritative_hardware_noun
    } else {
        true
    };

    let host_inspection_mode =
        host_inspection_allowed && preferred_host_inspection_topic(&lower).is_some();
    let maintainer_workflow_mode = preferred_maintainer_workflow(&lower).is_some();
    let workspace_workflow_mode =
        preferred_workspace_workflow(&lower).is_some() && !maintainer_workflow_mode;
    let toolchain_mode = contains_any(
        &lower,
        &[
            "tooling discipline",
            "best read-only toolchain",
            "identify the best tools you actually have",
            "concrete read-only investigation plan",
            "do not execute the plan",
            "available repo-inspection tools",
            "tool choice discipline",
            "what tools would you choose first",
        ],
    ) || (lower.contains("which tools") && lower.contains("why"))
        || (lower.contains("when would you choose") && lower.contains("tool"));
    let architecture_overview_mode = {
        let architecture_signals = contains_any(
            &lower,
            &[
                "architecture overview",
                "architecture walkthrough",
                "full architecture",
                "runtime walkthrough",
                "control flow",
                "tool routing",
                "workflow modes",
                "repo map behavior",
                "mcp policy",
                "prompt budgeting",
                "compaction",
                "file ownership",
                "owner file",
                "project structure",
                "repository structure",
            ],
        );
        let broad = contains_any(
            &lower,
            &[
                "full detailed",
                "all in one answer",
                "concrete file ownership",
                "walk me through",
                "major runtime pieces",
                "which files own",
                "how",
                "explain",
                "overview",
            ],
        );
        (architecture_signals && broad)
            || (lower.contains("runtime")
                && lower.contains("workflow")
                && (lower.contains("architecture") || lower.contains("tool routing")))
            || mentions_broad_system_walkthrough(&lower)
    };

    let direct_answer = if lower == "/help"
        || lower == "help"
        || lower == "/inventory"
        || lower == "/commands"
    {
        Some(DirectAnswerKind::Help)
    } else if lower == "/about"
        || lower == "/version"
        || lower == "about"
        || lower == "version"
        || mentions_creator_question(&lower)
    {
        Some(DirectAnswerKind::About)
    } else if matches!(
        lower.trim(),
        "who are you"
            | "who are you?"
            | "what are you"
            | "what are you?"
            | "what is your purpose"
            | "what is your purpose?"
            | "what's your purpose"
            | "what's your purpose?"
            | "what are you for"
            | "what are you for?"
            | "what is your job"
            | "what is your job?"
            | "what's your job"
            | "what's your job?"
    ) || (lower.contains("what is hematite") && !lower.contains("lm studio"))
    {
        Some(DirectAnswerKind::Identity)
    } else if (mentions_stable_product_surface(&lower) || mentions_product_truth_routing(&lower))
        && contains_any(
            &lower,
            &[
                "how hematite answers",
                "how does hematite answer",
                "how hematite handles",
                "how does hematite handle",
                "how hematite decides",
                "how does hematite decide",
                "decides whether",
                "decide whether",
            ],
        )
    {
        Some(DirectAnswerKind::ProductSurface)
    } else if mentions_reset_commands(&lower)
        && contains_any(
            &lower,
            &[
                "exact difference",
                "difference between",
                "explain the exact difference",
                "what is the difference",
            ],
        )
    {
        Some(DirectAnswerKind::SessionResetSemantics)
    } else if (lower.contains("reasoning output") || lower.contains("reasoning"))
        && contains_any(
            &lower,
            &["visible chat output", "visible chat", "chat output"],
        )
    {
        Some(DirectAnswerKind::ReasoningSplit)
    } else if lower.contains("/ask")
        && lower.contains("/code")
        && lower.contains("/architect")
        && lower.contains("/read-only")
        && lower.contains("/auto")
        && contains_any(&lower, &["difference", "differences", "what are"])
    {
        Some(DirectAnswerKind::WorkflowModes)
    } else if lower.contains(".hematite/settings.json")
        && lower.contains("gemma_native_auto")
        && lower.contains("gemma_native_formatting")
    {
        Some(DirectAnswerKind::GemmaNativeSettings)
    } else if contains_any(
        &lower,
        &[
            "skip verification",
            "skip build verification",
            "commit it immediately",
            "commit immediately",
        ],
    ) && contains_any(
        &lower,
        &[
            "make a code change",
            "make the change",
            "change the code",
            "edit the code",
            "edit a file",
            "implement",
        ],
    ) {
        Some(DirectAnswerKind::UnsafeWorkflowPressure)
    } else if contains_any(&lower, &["/gemma-native", "gemma native"])
        && contains_any(&lower, &["what does", "what is", "how does", "what do"])
    {
        Some(DirectAnswerKind::GemmaNative)
    } else if lower.contains("verify_build")
        && lower.contains(".hematite/settings.json")
        && contains_any(
            &lower,
            &["build", "test", "lint", "fix", "verification commands"],
        )
    {
        Some(DirectAnswerKind::VerifyProfiles)
    } else if (lower.contains("carry forward by default")
        || lower.contains("session memory should you carry forward")
        || (lower.contains("carry forward")
            && contains_any(
                &lower,
                &[
                    "besides the active task",
                    "blocker",
                    "compacts",
                    "recovers from a blocker",
                    "session state",
                ],
            )))
        && contains_any(
            &lower,
            &[
                "restarted hematite",
                "restarted",
                "avoid carrying forward",
                "session state",
                "active task",
                "blocker",
                "compacts",
                "recovers from a blocker",
            ],
        )
    {
        Some(DirectAnswerKind::SessionMemory)
    } else if contains_any(
        &lower,
        &[
            "recovery recipe",
            "recovery recipes",
            "recovery step",
            "recovery steps",
        ],
    ) && contains_any(
        &lower,
        &[
            "blocker",
            "runtime failure",
            "degrades",
            "context window",
            "context-window",
            "operator",
        ],
    ) {
        Some(DirectAnswerKind::RecoveryRecipes)
    } else if !architecture_overview_mode
        && contains_any(
            &lower,
            &[
                "mcp server health",
                "mcp runtime state",
                "mcp lifecycle",
                "mcp state",
                "mcp healthy",
                "mcp degraded",
                "mcp failed",
            ],
        )
    {
        Some(DirectAnswerKind::McpLifecycle)
    } else if contains_any(
        &lower,
        &[
            "allowed, denied, or require approval",
            "allowed denied or require approval",
            "allow, ask, or deny",
            "tool call should be allowed",
            "authorization logic",
            "workspace trust",
            "trust-allowlisted",
        ],
    ) {
        Some(DirectAnswerKind::AuthorizationPolicy)
    } else if contains_any(
        &lower,
        &[
            "tool classes",
            "tool class",
            "flat tool list",
            "runtime tool classes",
            "different runtime tool classes",
        ],
    ) || (lower.contains("repo reads")
        && lower.contains("repo writes")
        && contains_any(
            &lower,
            &[
                "verification tools",
                "git tools",
                "external mcp tools",
                "different runtime",
            ],
        ))
    {
        Some(DirectAnswerKind::ToolClasses)
    } else if contains_any(
        &lower,
        &[
            "built-in tool catalog",
            "builtin tool catalog",
            "builtin-tool dispatch",
            "built-in tool dispatch",
            "tool registry ownership",
            "which file now owns",
        ],
    ) && contains_any(
        &lower,
        &[
            "tool catalog",
            "dispatch path",
            "dispatch",
            "tool registry",
            "owns",
        ],
    ) {
        Some(DirectAnswerKind::ToolRegistryOwnership)
    } else if (lower.contains("other coding languages")
        || lower.contains("what languages")
        || lower.contains("know other languages"))
        && contains_any(
            &lower,
            &[
                "capable of making projects",
                "can you make projects",
                "can you build projects",
            ],
        )
    {
        Some(DirectAnswerKind::LanguageCapability)
    } else if workflow_mode == WorkflowMode::Architect
        && (lower.contains("session reset")
            || (lower.contains("/clear") && lower.contains("/new") && lower.contains("/forget")))
        && contains_any(&lower, &["redesign", "clearer", "easier", "understand"])
    {
        Some(DirectAnswerKind::ArchitectSessionResetPlan)
    } else if toolchain_mode
        && lower.contains("read-only")
        && contains_any(
            &lower,
            &[
                "tooling discipline",
                "investigation plan",
                "best read-only toolchain",
                "tool choice discipline",
                "what tools would you choose first",
            ],
        )
    {
        Some(DirectAnswerKind::Toolchain)
    } else if !architecture_overview_mode
        && host_inspection_mode
        && mentions_host_inspection_question(&lower)
    {
        Some(DirectAnswerKind::HostInspection)
    } else {
        None
    };

    let sovereign_mode = is_sovereign_mutation(user_input);

    let primary_class = if architecture_overview_mode {
        QueryIntentClass::RepoArchitecture
    } else if direct_answer.is_some()
        || mentions_stable_product_surface(&lower)
        || mentions_product_truth_routing(&lower)
    {
        QueryIntentClass::ProductTruth
    } else if mentions_research_query(&lower) {
        // Disambiguation: if also mentions codebase keywords, it's likely a local search.
        if mentions_codebase_keywords(&lower) {
            if lower.contains("logic") || lower.contains("wiring") || lower.contains("architecture")
            {
                QueryIntentClass::RepoArchitecture
            } else {
                QueryIntentClass::RuntimeDiagnosis
            }
        } else {
            QueryIntentClass::Research
        }
    } else if toolchain_mode {
        QueryIntentClass::Toolchain
    } else if capability_mode {
        QueryIntentClass::Capability
    } else if mentions_runtime_trace || anti_guess || lower.contains("read-only") {
        QueryIntentClass::RuntimeDiagnosis
    } else if looks_like_mutation_request(user_input) {
        QueryIntentClass::Implementation
    } else {
        QueryIntentClass::Unknown
    };

    QueryIntent {
        primary_class,
        direct_answer,
        grounded_trace_mode: mentions_runtime_trace || lower.contains("read-only") || anti_guess,
        capability_mode,
        capability_needs_repo,
        toolchain_mode,
        host_inspection_mode,
        maintainer_workflow_mode: maintainer_workflow_mode && !sovereign_mode,
        workspace_workflow_mode: workspace_workflow_mode && !sovereign_mode,
        architecture_overview_mode,
        sovereign_mode,
        surgical_filesystem_mode: is_simple_surgical_filesystem_request(user_input),
        scaffold_mode: is_scaffold_request(user_input),
    }
}

pub fn is_scaffold_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();

    // Creation/generation verbs (combined with stack keywords for specificity)
    let creation_verbs = contains_any(
        &lower,
        &[
            "scaffold",
            "bootstrap",
            "create a",
            "create an",
            "create me a",
            "create me an",
            "make a",
            "make an",
            "make me a",
            "make me an",
            "build a",
            "build an",
            "build me a",
            "build me an",
            "generate a",
            "generate an",
            "set up a",
            "set up an",
            "set me up a",
            "set me up an",
            "spin up a",
            "spin up an",
            "start a",
            "start an",
            "init a",
            "init an",
            "initialize a",
            "initialize an",
            "write a",
            "write me a",
            "write me an",
            "build website",
            "make website",
            "create website",
            "scaffold website",
        ],
    );

    // Stack/project keywords — broad enough to catch short requests like "make me a rust app"
    let stack_keywords = contains_any(
        &lower,
        &[
            // Web frameworks
            "react app",
            "react project",
            "react site",
            "react component",
            "next.js",
            "nextjs",
            "next app",
            "next project",
            "nuxt",
            "vue app",
            "vue project",
            "vue site",
            "vue component",
            "svelte app",
            "svelte project",
            "sveltekit",
            "astro project",
            "astro site",
            "remix app",
            "solid.js",
            // Backend
            "express app",
            "express server",
            "express api",
            "express project",
            "fastapi",
            "flask app",
            "flask project",
            "flask api",
            "django project",
            "django app",
            "node project",
            "node app",
            "node server",
            "node api",
            "typescript project",
            "ts project",
            "ts app",
            // Rust
            "rust cli",
            "rust project",
            "rust app",
            "rust tool",
            "rust binary",
            "rust library",
            "rust crate",
            "rust api",
            // Go
            "go project",
            "go app",
            "go cli",
            "go api",
            "go server",
            "go tool",
            "golang project",
            "golang app",
            // Python
            "python project",
            "python app",
            "python cli",
            "python script",
            "python package",
            "python tool",
            "python api",
            "python service",
            "python library",
            // C / C++
            "c++ project",
            "c++ app",
            "cpp project",
            "cpp app",
            "c project",
            "c app",
            "cmake project",
            // Generic project types
            "landing page",
            "html website",
            "html site",
            "html page",
            "html file",
            "single file html",
            "single-file html",
            "single html file",
            "single index.html",
            "index.html",
            "portfolio site",
            "portfolio page",
            "personal site",
            "todo app",
            "rest api",
            "graphql api",
            "crud app",
            "web app",
            "web project",
            "web site",
            "website",
            "cli app",
            "cli tool",
            "command line tool",
            "command-line tool",
            "desktop app",
            "mobile app",
            "microservice",
            "api server",
            "backend api",
            "new project",
            "new app",
            "new site",
        ],
    );

    // Explicit scaffold tool invocations (always scaffold regardless of verb)
    let scaffold_commands = contains_any(
        &lower,
        &[
            "npm init",
            "npm create",
            "cargo new",
            "cargo init",
            "go mod init",
            "npx create-react-app",
            "npx create-next-app",
            "npx create-vue",
            "npx create-svelte",
            "npx astro",
            "pnpm create",
            "yarn create",
            "django-admin startproject",
            "python -m django startproject",
        ],
    );

    (creation_verbs && stack_keywords) || scaffold_commands
}

fn is_simple_surgical_filesystem_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let mentions_creation = contains_any(
        &lower,
        &[
            "make a folder",
            "make a directory",
            "make a file",
            "create a folder",
            "create a directory",
            "create a file",
            "new folder",
            "new directory",
        ],
    );
    let mentions_sovereign = contains_any(
        &lower,
        &[
            "@desktop",
            "@documents",
            "@downloads",
            "@home",
            "~/",
            "@temp",
        ],
    );

    mentions_creation || mentions_sovereign
}

pub(crate) fn is_capability_probe_tool(name: &str) -> bool {
    matches!(
        name,
        "read_file"
            | "inspect_lines"
            | "list_files"
            | "grep_files"
            | "lsp_definitions"
            | "lsp_references"
            | "lsp_hover"
            | "lsp_search_symbol"
            | "lsp_get_diagnostics"
            | "trace_runtime_flow"
            | "auto_pin_context"
            | "list_pinned"
    )
}

/// Returns true when the user's query is GitHub-related and should use `github_ops`.
/// The model should never shell out to `gh` — use the dedicated tool instead.
pub fn needs_github_ops(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("pull request")
        || lower.contains("open pr")
        || lower.contains("create pr")
        || lower.contains("merge pr")
        || lower.contains("list prs")
        || lower.contains("list issues")
        || lower.contains("open issue")
        || lower.contains("create issue")
        || lower.contains("github issue")
        || lower.contains("ci status")
        || lower.contains("ci run")
        || lower.contains("github actions")
        || lower.contains("workflow run")
        || lower.contains("gh pr")
        || lower.contains("gh issue")
        || lower.contains("gh run")
        || (lower.contains("check") && lower.contains("pr"))
        || (lower.contains("status") && lower.contains("ci"))
}

/// Returns true when the user's query involves crash/panic/segfault debugging that should
/// use run_with_backtrace instead of shell for structured RUST_BACKTRACE output.
pub fn needs_crash_debug(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    (lower.contains("crash") || lower.contains("panic") || lower.contains("panicked"))
        || lower.contains("segfault")
        || lower.contains("segmentation fault")
        || lower.contains("access violation")
        || lower.contains("stack overflow")
        || lower.contains("backtrace")
        || lower.contains("stack trace")
        || lower.contains("why does it crash")
        || lower.contains("why is it crashing")
        || lower.contains("debug the crash")
        || lower.contains("debug crash")
        || (lower.contains("run") && lower.contains("backtrace"))
        || (lower.contains("get") && lower.contains("backtrace"))
        || lower.contains("core dump")
        || lower.contains("aborted")
        || lower.contains("fatal runtime")
        || lower.contains("sigsegv")
        || lower.contains("sigabrt")
}

/// Returns true when the user's query is about formatting code — steer toward `format_code`
/// instead of raw `shell cargo fmt`.
pub fn needs_format(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("cargo fmt")
        || lower.contains("rustfmt")
        || lower.contains("prettier")
        || lower.contains("black format")
        || lower.contains("ruff format")
        || lower.contains("format the code")
        || lower.contains("format code")
        || lower.contains("format this file")
        || lower.contains("format the file")
        || lower.contains("run fmt")
        || lower.contains("run the formatter")
        || lower.contains("apply formatting")
        || lower.contains("check formatting")
        || lower.contains("is the code formatted")
        || lower.contains("needs formatting")
        || (lower.contains("format") && lower.contains("rust"))
        || (lower.contains("format") && lower.contains("python"))
        || (lower.contains("format") && lower.contains("typescript"))
}

/// Returns true when the user's query is about linting — steer toward `lint_code`
/// instead of raw `shell cargo clippy`.
pub fn needs_lint_check(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("clippy")
        || lower.contains("lint")
        || lower.contains("linting")
        || lower.contains("warnings")
        || lower.contains("dead code")
        || lower.contains("unused import")
        || lower.contains("unused variable")
        || lower.contains("fix clippy")
        || lower.contains("fix warnings")
        || lower.contains("fix lints")
        || lower.contains("apply clippy")
        || lower.contains("clippy fix")
        || lower.contains("cargo clippy")
        || (lower.contains("check") && lower.contains("lint"))
        || (lower.contains("run") && lower.contains("clippy"))
}

/// Returns true when the user's query is asking to run tests — steer toward `run_tests`
/// instead of raw `shell cargo test` or `shell pytest`.
pub fn needs_test_run(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let test_noun = lower.contains("test") || lower.contains("spec") || lower.contains("suite");
    let run_verb = lower.contains("run")
        || lower.contains("execute")
        || lower.contains("check failing")
        || lower.contains("failing test")
        || lower.contains("flaky test")
        || lower.contains("re-run")
        || lower.contains("rerun");
    let explicit = lower.contains("cargo test")
        || lower.contains("pytest")
        || lower.contains("npm test")
        || lower.contains("run the tests")
        || lower.contains("run all tests")
        || lower.contains("run tests")
        || lower.contains("run failing tests")
        || lower.contains("run this test")
        || lower.contains("run the test suite")
        || lower.contains("test suite")
        || lower.contains("test results")
        || lower.contains("which tests fail")
        || lower.contains("what tests fail")
        || lower.contains("tests pass")
        || lower.contains("tests fail")
        || lower.contains("test is failing")
        || lower.contains("test failed");
    explicit || (run_verb && test_noun)
}

/// Returns true when the user's query is about making an HTTP request — steer toward `http_request`.
pub fn needs_http_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("http request")
        || lower.contains("make a request")
        || lower.contains("send a request")
        || lower.contains("send a get")
        || lower.contains("send a post")
        || lower.contains("send a put")
        || lower.contains("send a delete")
        || lower.contains("send a patch")
        || lower.contains("curl the")
        || lower.contains("call the api")
        || lower.contains("call this api")
        || lower.contains("call this endpoint")
        || lower.contains("call the endpoint")
        || lower.contains("hit the endpoint")
        || lower.contains("hit the api")
        || lower.contains("fetch this url")
        || lower.contains("fetch that url")
        || lower.contains("fetch the url")
        || lower.contains("test the api")
        || lower.contains("test this endpoint")
        || lower.contains("test the endpoint")
        || lower.contains("send this payload")
        || lower.contains("post to")
        || lower.contains("get request")
        || lower.contains("post request")
        || (lower.contains("make")
            && (lower.contains("get request") || lower.contains("post request")))
        || (lower.contains("api") && lower.contains("request"))
}

/// Returns true when the user's query is about Docker — steer toward `docker_ops`.
pub fn needs_docker_compose_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "docker-compose.yml",
            "docker-compose.yaml",
            "compose.yml",
            "compose.yaml",
            "parse docker-compose",
            "parse compose",
            "docker compose services",
            "services in docker-compose",
            "docker compose ports",
            "docker compose volumes",
            "docker compose networks",
            "compose file",
            "explain docker-compose",
            "validate docker-compose",
            "docker compose env",
            "compose service",
        ],
    )
}

pub fn needs_dockerfile_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "dockerfile",
            "docker file",
            "parse dockerfile",
            "validate dockerfile",
            "review dockerfile",
            "dockerfile layers",
            "dockerfile best practices",
            "from instruction",
            "cmd instruction",
            "entrypoint instruction",
            "healthcheck instruction",
            "docker image build",
        ],
    )
}

pub fn needs_docker_ops(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let docker_noun = lower.contains("docker")
        || lower.contains("container")
        || lower.contains("compose")
        || lower.contains("dockerfile");
    let action_verb = lower.contains("list")
        || lower.contains("show")
        || lower.contains("start")
        || lower.contains("stop")
        || lower.contains("restart")
        || lower.contains("remove")
        || lower.contains("pull")
        || lower.contains("build")
        || lower.contains("logs")
        || lower.contains("running")
        || lower.contains("up")
        || lower.contains("down")
        || lower.contains("inspect")
        || lower.contains("stats");
    let explicit = lower.contains("docker ps")
        || lower.contains("docker logs")
        || lower.contains("docker images")
        || lower.contains("docker stats")
        || lower.contains("docker compose")
        || lower.contains("docker-compose")
        || lower.contains("running containers")
        || lower.contains("which containers")
        || lower.contains("what containers");
    explicit || (docker_noun && action_verb)
}

/// Returns true when the user wants to scan for secrets/credentials — steer toward `secret_scanner`.
pub fn needs_secret_scan(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("secret scan")
        || lower.contains("scan for secret")
        || lower.contains("scan for credential")
        || lower.contains("scan for api key")
        || lower.contains("scan for token")
        || lower.contains("leaked secret")
        || lower.contains("leaked credential")
        || lower.contains("leaked key")
        || lower.contains("leaked api key")
        || lower.contains("leaked token")
        || lower.contains("committed secret")
        || lower.contains("committed credential")
        || lower.contains("check for secret")
        || lower.contains("check for credential")
        || lower.contains("find secret")
        || lower.contains("find credential")
        || lower.contains("find api key")
        || lower.contains("detect secret")
        || lower.contains("detect credential")
        || lower.contains("hardcoded secret")
        || lower.contains("hardcoded credential")
        || lower.contains("hardcoded password")
        || lower.contains("hardcoded key")
        || lower.contains("hardcoded token")
        || lower.contains("exposed secret")
        || lower.contains("exposed credential")
        || lower.contains("exposed api key")
        || lower.contains("any secrets in")
        || lower.contains("any credentials in")
        || lower.contains("sensitive data in")
        || (lower.contains("secret") && lower.contains("repo"))
        || (lower.contains("secret") && lower.contains("codebase"))
        || (lower.contains("credential") && lower.contains("repo"))
        || (lower.contains("credential") && lower.contains("codebase"))
        || (lower.contains("api key") && (lower.contains("committed") || lower.contains("expose")))
        || lower.contains("gitleaks")
        || lower.contains("trufflehog")
}

/// Returns true when the user wants to diff or compare text/files — steer toward `diff_tools`.
pub fn needs_diff_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("diff these")
        || lower.contains("diff the")
        || lower.contains("diff two files")
        || lower.contains("compare these files")
        || lower.contains("compare the files")
        || lower.contains("compare these two")
        || lower.contains("show me the diff")
        || lower.contains("show the diff")
        || lower.contains("generate a patch")
        || lower.contains("make a patch")
        || lower.contains("apply this patch")
        || lower.contains("apply the patch")
        || lower.contains("word diff")
        || lower.contains("word-diff")
        || lower.contains("diff stat")
        || lower.contains("how many lines changed")
        || lower.contains("what changed between")
        || lower.contains("what's the difference between")
        || lower.contains("what is the difference between")
        || (lower.contains("diff") && lower.contains("file"))
        || (lower.contains("diff") && lower.contains("config"))
        || (lower.contains("unified diff") || lower.contains("unified patch"))
        || (lower.contains("compare") && lower.contains("versions"))
        || (lower.contains("similarity") && lower.contains("files"))
}

/// Returns true when the user wants regex help — steer toward `regex_tools`.
pub fn needs_regex_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("test this regex")
        || lower.contains("test regex")
        || lower.contains("test my regex")
        || lower.contains("test pattern")
        || lower.contains("regex match")
        || lower.contains("does this pattern match")
        || lower.contains("does my regex")
        || lower.contains("extract with regex")
        || lower.contains("extract using regex")
        || lower.contains("regex extract")
        || lower.contains("replace with regex")
        || lower.contains("replace using regex")
        || lower.contains("regex replace")
        || lower.contains("regex split")
        || lower.contains("split on regex")
        || lower.contains("split with regex")
        || lower.contains("explain this regex")
        || lower.contains("explain regex")
        || lower.contains("what does this regex")
        || lower.contains("what does this pattern")
        || lower.contains("named capture group")
        || lower.contains("named groups")
        || lower.contains("regex named")
        || (lower.contains("regex") && lower.contains("pattern") && lower.contains("test"))
        || (lower.contains("regular expression") && lower.contains("test"))
        || (lower.contains("regular expression") && lower.contains("match"))
}

/// Returns true when the user's query involves computation that must be exact —
/// checksums, financial math, statistics, date arithmetic, algorithmic verification, etc.
/// Used by the harness to inject a pre-turn nudge toward run_code instead of model memory.
pub fn needs_computation_sandbox(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();

    // ── Computation verb set (reused by multiple categories) ─────────────────
    let has_compute_verb = lower.contains("calculat")
        || lower.contains("compute")
        || lower.contains("what is")
        || lower.contains("what's")
        || lower.contains("how much is")
        || lower.contains("how much does")
        || lower.contains("solve")
        || lower.contains("evaluate")
        || lower.contains("find the")
        || lower.contains("work out");

    // ── Hash / checksum ───────────────────────────────────────────────────────
    let hash_or_checksum = lower.contains("sha")
        || lower.contains("md5")
        || lower.contains("checksum")
        || lower.contains("crc")
        || lower.contains("hash")
        || lower.contains("fingerprint");

    // ── Simple arithmetic (any inline operator with digits) ───────────────────
    let simple_arithmetic = {
        let has_operator = lower.contains(" + ")
            || lower.contains(" - ")
            || lower.contains(" * ")
            || lower.contains(" / ")
            || lower.contains(" × ")
            || lower.contains(" ÷ ")
            || lower.contains(" ^ ")
            || lower.contains("squared")
            || lower.contains("cubed")
            || lower.contains("times ")
            || lower.contains("divided by")
            || lower.contains("multiplied by")
            || lower.contains("plus ")
            || lower.contains("minus ");
        let has_digit = lower.chars().any(|c| c.is_ascii_digit());
        has_operator && has_digit && has_compute_verb
    };

    // ── Financial / percentage ────────────────────────────────────────────────
    let financial = has_compute_verb
        && (lower.contains("percent")
            || lower.contains("%")
            || lower.contains("interest")
            || lower.contains("compound")
            || lower.contains("roi")
            || lower.contains("tax")
            || lower.contains("discount")
            || lower.contains("profit")
            || lower.contains("loss")
            || lower.contains("salary")
            || lower.contains("annuali")
            || lower.contains("amortiz")
            || lower.contains("mortgage")
            || lower.contains("exchange rate")
            || lower.contains("currency"));

    // ── Statistics / data analysis ────────────────────────────────────────────
    let statistics = lower.contains("standard deviation")
        || lower.contains("std dev")
        || lower.contains("mean of")
        || lower.contains("median of")
        || lower.contains("average of")
        || lower.contains("variance of")
        || lower.contains("regression")
        || lower.contains("correlation")
        || lower.contains("percentile")
        || lower.contains("quartile")
        || lower.contains("sum of")
        || lower.contains("total of")
        || lower.contains("count of")
        || lower.contains("these numbers")
        || lower.contains("the following numbers")
        || lower.contains("the following data")
        || lower.contains("from the data")
        || lower.contains("in this dataset")
        || lower.contains("from this csv")
        || lower.contains("from this table")
        || lower.contains("analyze the data")
        || lower.contains("analyze this data")
        || lower.contains("analyze these numbers");

    // ── Geometry / trigonometry ───────────────────────────────────────────────
    let geometry = lower.contains("area of")
        || lower.contains("volume of")
        || lower.contains("circumference")
        || lower.contains("perimeter of")
        || lower.contains("hypotenuse")
        || lower.contains("pythagorean")
        || lower.contains("square root")
        || lower.contains("sqrt")
        || lower.contains("cube root")
        || (has_compute_verb
            && (lower.contains(" sine ")
                || lower.contains(" sin ")
                || lower.contains(" cosine ")
                || lower.contains(" cos ")
                || lower.contains(" tangent ")
                || lower.contains(" tan ")))
        || lower.contains("logarithm")
        || lower.contains("log base")
        || lower.contains("natural log")
        || lower.contains(" ln ")
        || (has_compute_verb
            && (lower.contains("exponent")
                || lower.contains("power of")
                || lower.contains("to the power")
                || lower.contains("raised to")))
        || (has_compute_verb && lower.contains("derivative"))
        || (has_compute_verb && lower.contains("integral"));

    // ── Date / time arithmetic ────────────────────────────────────────────────
    let date_math = (lower.contains("how many days")
        || lower.contains("how many hours")
        || lower.contains("how many weeks")
        || lower.contains("how many months")
        || lower.contains("days between")
        || lower.contains("hours between")
        || lower.contains("weeks between")
        || lower.contains("days until")
        || lower.contains("days since")
        || lower.contains("unix timestamp")
        || lower.contains("epoch")
        || lower.contains("time zone")
        || lower.contains("timezone"))
        && (lower.contains("date")
            || lower.contains("day")
            || lower.contains("hour")
            || lower.contains("week")
            || lower.contains("month")
            || lower.contains("timestamp")
            || lower.contains("time"));

    // ── Algorithms / code execution ───────────────────────────────────────────
    let algorithmic = lower.contains("is prime")
        || lower.contains("prime number")
        || lower.contains("factori")
        || lower.contains("fibonacci")
        || lower.contains("factorial")
        || lower.contains("sort this")
        || lower.contains("verify this algorithm")
        || lower.contains("run this code")
        || lower.contains("execute this")
        || lower.contains("big-o")
        || lower.contains("time complexity")
        || lower.contains("space complexity");

    // ── Unit conversion ───────────────────────────────────────────────────────
    let unit_conversion = (lower.contains("convert") || lower.contains("how many"))
        && (lower.contains(" bytes")
            || lower.contains(" kb")
            || lower.contains(" mb")
            || lower.contains(" gb")
            || lower.contains(" tb")
            || lower.contains("gigabyte")
            || lower.contains("megabyte")
            || lower.contains("celsius")
            || lower.contains("fahrenheit")
            || lower.contains("kelvin")
            || lower.contains("kilometers")
            || lower.contains("kilometres")
            || lower.contains("miles")
            || lower.contains("meters")
            || lower.contains("metres")
            || lower.contains("feet")
            || lower.contains("inches")
            || lower.contains("centimeter")
            || lower.contains("centimetre")
            || lower.contains("pounds")
            || lower.contains("kilograms")
            || lower.contains("ounces")
            || lower.contains("liters")
            || lower.contains("litres")
            || lower.contains("gallons")
            || lower.contains("watts")
            || lower.contains("kilowatts")
            || lower.contains("volts")
            || lower.contains("ampere")
            || lower.contains("horsepower"));

    hash_or_checksum
        || simple_arithmetic
        || financial
        || statistics
        || date_math
        || algorithmic
        || unit_conversion
        || geometry
}

/// Returns true when the user wants to work with YAML — steer toward `yaml_tools`.
pub fn needs_yaml_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("validate yaml")
        || lower.contains("parse yaml")
        || lower.contains("format yaml")
        || lower.contains("yaml file")
        || lower.contains("yaml path")
        || lower.contains("yaml key")
        || lower.contains("yaml to json")
        || lower.contains("json to yaml")
        || lower.contains("merge yaml")
        || lower.contains("diff yaml")
        || lower.contains("yaml diff")
        || lower.contains("get from yaml")
        || lower.contains("read yaml")
        || lower.contains("check yaml")
        || lower.contains("yaml is valid")
        || lower.contains("is this yaml")
        || lower.contains("kubernetes yaml")
        || lower.contains("helm chart")
        || lower.contains("docker-compose yaml")
        || lower.contains("ansible yaml")
}

/// Returns true when the user wants to work with CSV data — steer toward `csv_tools`.
pub fn needs_csv_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("csv file")
        || lower.contains("read csv")
        || lower.contains("parse csv")
        || lower.contains("csv column")
        || lower.contains("csv row")
        || lower.contains("csv header")
        || lower.contains("csv stats")
        || lower.contains("csv filter")
        || lower.contains("csv sort")
        || lower.contains("csv to json")
        || lower.contains("csv to markdown")
        || lower.contains("from csv")
        || lower.contains("analyse csv")
        || lower.contains("analyze csv")
        || lower.contains("count rows")
        || lower.contains("columns in csv")
        || lower.contains("head of csv")
        || lower.contains("preview csv")
}

/// Returns true when the user wants encoding/decoding operations — steer toward `encode_tools`.
pub fn needs_encode_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("base64")
        || lower.contains("url encode")
        || lower.contains("url decode")
        || lower.contains("urlencode")
        || lower.contains("urldecode")
        || lower.contains("percent encode")
        || lower.contains("percent decode")
        || lower.contains("hex encode")
        || lower.contains("hex decode")
        || lower.contains("encode to hex")
        || lower.contains("decode hex")
        || lower.contains("decode jwt")
        || lower.contains("jwt decode")
        || lower.contains("parse jwt")
        || lower.contains("jwt token")
        || lower.contains("html encode")
        || lower.contains("html decode")
        || lower.contains("html entity")
        || lower.contains("escape html")
        || lower.contains("unescape html")
        || lower.contains("encode this")
        || lower.contains("decode this")
}

pub fn needs_har_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".har file",
            "har file",
            "http archive",
            "parse har",
            "analyze har",
            "har entries",
            "slowest requests",
            "web performance",
            "network waterfall",
            "request timing",
            "har summary",
            "response errors",
            "http requests log",
            "browser network log",
            "chrome devtools export",
        ],
    )
}

pub fn needs_ical_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".ics file",
            "ical file",
            "icalendar",
            "parse ical",
            "parse ics",
            "calendar events",
            "vevent",
            "vtodo",
            "calendar file",
            "ics calendar",
            "recurring event",
            "calendar entry",
            "outlook calendar export",
            "google calendar export",
        ],
    )
}

/// Returns true when the user wants to hash data — steer toward `hash_tools`.
pub fn needs_hash_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("sha256")
        || lower.contains("sha-256")
        || lower.contains("sha512")
        || lower.contains("sha-512")
        || lower.contains("sha2")
        || lower.contains("md5 hash")
        || lower.contains("md5 of")
        || lower.contains("hash this")
        || lower.contains("hash the file")
        || lower.contains("hash of this")
        || lower.contains("checksum this")
        || lower.contains("digest of")
        || lower.contains("hmac")
        || lower.contains("compute hash")
        || lower.contains("generate hash")
        || lower.contains("file hash")
        || lower.contains("string hash")
        || lower.contains("hash string")
        || lower.contains("cryptographic hash")
}

/// Returns true when the user wants to work with TOML — steer toward `toml_tools`.
pub fn needs_toml_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("toml file")
        || lower.contains("parse toml")
        || lower.contains("validate toml")
        || lower.contains("read toml")
        || lower.contains("check toml")
        || lower.contains("toml to json")
        || lower.contains("json to toml")
        || lower.contains("format toml")
        || lower.contains("toml key")
        || lower.contains("toml path")
        || lower.contains("get from toml")
        || lower.contains("cargo.toml key")
        || lower.contains("cargo.toml value")
        || lower.contains("is this toml")
        || lower.contains("toml is valid")
}

/// Returns true when the user wants text manipulation — steer toward `text_tools`.
pub fn needs_text_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("snake_case")
        || lower.contains("camelcase")
        || lower.contains("camel case")
        || lower.contains("pascalcase")
        || lower.contains("pascal case")
        || lower.contains("kebab-case")
        || lower.contains("kebab case")
        || lower.contains("screaming snake")
        || lower.contains("convert to snake")
        || lower.contains("convert to camel")
        || lower.contains("convert to pascal")
        || lower.contains("convert to kebab")
        || lower.contains("to snake case")
        || lower.contains("to camel case")
        || lower.contains("to pascal case")
        || lower.contains("to kebab case")
        || lower.contains("slugify")
        || lower.contains("make a slug")
        || lower.contains("url slug")
        || lower.contains("word count")
        || lower.contains("count words")
        || lower.contains("count characters")
        || lower.contains("char count")
        || lower.contains("line count")
        || lower.contains("count lines")
        || lower.contains("truncate text")
        || lower.contains("truncate this")
        || lower.contains("word wrap")
        || lower.contains("wrap text")
        || lower.contains("wrap at")
        || lower.contains("pad string")
        || lower.contains("pad text")
        || lower.contains("sort lines")
        || lower.contains("dedupe lines")
        || lower.contains("reverse string")
        || lower.contains("reverse text")
}

/// Returns true when the user wants date/time work — steer toward `date_tools`.
pub fn needs_date_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("what time is it")
        || lower.contains("what's the date")
        || lower.contains("what is the date")
        || lower.contains("current date")
        || lower.contains("current time")
        || lower.contains("date now")
        || lower.contains("unix timestamp")
        || lower.contains("unix epoch")
        || lower.contains("from timestamp")
        || lower.contains("timestamp to date")
        || lower.contains("epoch to date")
        || lower.contains("date to timestamp")
        || lower.contains("date format")
        || lower.contains("format date")
        || lower.contains("format this date")
        || lower.contains("parse date")
        || lower.contains("parse this date")
        || lower.contains("add days")
        || lower.contains("add weeks")
        || lower.contains("add months")
        || lower.contains("add years")
        || (lower.contains("add") && lower.contains("months") && lower.contains("to"))
        || lower.contains("date diff")
        || lower.contains("days between")
        || lower.contains("days until")
        || lower.contains("days since")
        || lower.contains("how many days")
        || lower.contains("date difference")
        || lower.contains("relative date")
        || lower.contains("time ago")
        || lower.contains("how long ago")
        || lower.contains("what day of the week")
        || lower.contains("what weekday")
        || lower.contains("which day")
        || lower.contains("iso 8601")
        || lower.contains("rfc 3339")
        || lower.contains("week number")
}

/// Returns true when the user wants number formatting or conversion — steer toward `number_tools`.
pub fn needs_number_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("roman numeral")
        || lower.contains("to roman")
        || lower.contains("from roman")
        || lower.contains("convert to hex")
        || lower.contains("convert to binary")
        || lower.contains("convert to octal")
        || (lower.contains("convert") && lower.contains("to hex"))
        || (lower.contains("convert") && lower.contains("to binary"))
        || lower.contains("hex to decimal")
        || lower.contains("binary to decimal")
        || lower.contains("decimal to hex")
        || lower.contains("decimal to binary")
        || lower.contains("base conversion")
        || lower.contains("number base")
        || lower.contains("convert base")
        || lower.contains("si prefix")
        || lower.contains("si unit")
        || lower.contains("kilobytes")
        || lower.contains("megabytes")
        || lower.contains("gigabytes")
        || lower.contains("prime factors")
        || lower.contains("prime factorization")
        || lower.contains("factorize")
        || lower.contains("is prime")
        || lower.contains("check prime")
        || lower.contains("gcd of")
        || lower.contains("lcm of")
        || lower.contains("greatest common")
        || lower.contains("least common multiple")
        || lower.contains("clamp number")
        || lower.contains("format number")
        || lower.contains("thousands separator")
        || lower.contains("engineering notation")
        || lower.contains("scientific notation")
        || lower.contains("number format")
}

/// Returns true when the user wants UUID generation or validation — steer toward `uuid_gen`.
pub fn needs_uuid_gen(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("uuid")
        || lower.contains("generate id")
        || lower.contains("generate an id")
        || lower.contains("unique id")
        || lower.contains("unique identifier")
        || lower.contains("guid")
        || lower.contains("validate uuid")
        || lower.contains("is this a uuid")
        || lower.contains("nil uuid")
        || lower.contains("bulk uuid")
        || lower.contains("multiple uuid")
}

/// Returns true when the user wants cron expression help — steer toward `cron_tools`.
pub fn needs_cron_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("cron")
        || lower.contains("crontab")
        || lower.contains("cron expression")
        || lower.contains("cron schedule")
        || lower.contains("explain this schedule")
        || lower.contains("next run")
        || lower.contains("when does this job run")
        || lower.contains("when will it run")
        || lower.contains("scheduled job")
        || lower.contains("0 * * * *")
        || lower.contains("*/")
        || (lower.contains("schedule") && lower.contains("explain"))
}

/// Returns true when the user wants IP address or CIDR tools — steer toward `ip_tools`.
pub fn needs_ip_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("cidr")
        || lower.contains("subnet mask")
        || lower.contains("subnet calculation")
        || lower.contains("ip address info")
        || lower.contains("ip address class")
        || lower.contains("network address")
        || lower.contains("broadcast address")
        || lower.contains("usable hosts")
        || lower.contains("is this ip")
        || lower.contains("ip range")
        || lower.contains("ip subnet")
        || lower.contains("convert ip")
        || lower.contains("ip to decimal")
        || lower.contains("ip in subnet")
        || lower.contains("ip contains")
        || lower.contains("192.168.")
        || lower.contains("10.0.")
        || lower.contains("/24")
        || lower.contains("/16")
        || lower.contains("/8")
}

/// Returns true when the user wants advanced subnet operations — steer toward `subnet_tools`.
pub fn needs_subnet_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("split subnet")
        || lower.contains("subnet split")
        || lower.contains("subnets from cidr")
        || lower.contains("divide cidr")
        || lower.contains("divide subnet")
        || lower.contains("list hosts in cidr")
        || lower.contains("enumerate hosts")
        || lower.contains("hosts in subnet")
        || lower.contains("hosts in cidr")
        || lower.contains("supernet")
        || lower.contains("cidr aggregat")
        || lower.contains("aggregate cidr")
        || lower.contains("aggregate subnet")
        || lower.contains("aggregate these cidr")
        || lower.contains("merge cidr")
        || lower.contains("cidr overlap")
        || lower.contains("overlapping cidr")
        || lower.contains("ip range to cidr")
        || lower.contains("cidr from range")
        || lower.contains("subnet_tools")
        || (lower.contains("split") && lower.contains("cidr") && lower.contains("subnet"))
        || (lower.contains("aggregate") && lower.contains("subnet"))
}

/// Returns true when the user wants color conversion or analysis — steer toward `color_tools`.
pub fn needs_color_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("hex color")
        || lower.contains("rgb color")
        || lower.contains("hsl color")
        || lower.contains("color hex")
        || lower.contains("color code")
        || lower.contains("convert color")
        || lower.contains("color conversion")
        || lower.contains("contrast ratio")
        || lower.contains("wcag")
        || lower.contains("color contrast")
        || lower.contains("lighten color")
        || lower.contains("darken color")
        || lower.contains("mix colors")
        || lower.contains("blend color")
        || lower.contains("color palette")
        || lower.contains("complementary color")
        || lower.contains("#rrggbb")
        || lower.contains("rgb(")
        || lower.contains("hsl(")
}

/// Returns true when the user wants semver parsing or comparison — steer toward `semver_tools`.
pub fn needs_semver_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("semver")
        || lower.contains("semantic version")
        || lower.contains("bump version")
        || lower.contains("bump the version")
        || lower.contains("compare versions")
        || lower.contains("version range")
        || lower.contains("satisfies range")
        || lower.contains("is compatible")
        || lower.contains("version compatible")
        || lower.contains("parse version")
        || lower.contains("sort versions")
        || lower.contains("which version is newer")
        || lower.contains("version constraint")
        || lower.contains("^1.")
        || lower.contains("~1.")
}

/// Returns true when the user wants password generation or strength analysis — steer toward `password_gen`.
pub fn needs_password_gen(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("generate password")
        || lower.contains("generate a password")
        || lower.contains("random password")
        || lower.contains("secure password")
        || lower.contains("strong password")
        || lower.contains("passphrase")
        || lower.contains("pass phrase")
        || lower.contains("password strength")
        || lower.contains("how strong is")
        || lower.contains("check password")
        || lower.contains("generate pin")
        || lower.contains("random pin")
        || lower.contains("numeric pin")
        || lower.contains("memorable password")
}

/// Returns true when the user wants JWT decode/verify/sign — steer toward `jwt_tools`.
pub fn needs_jwt_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("jwt")
        || lower.contains("json web token")
        || lower.contains("bearer token")
        || lower.contains("decode token")
        || lower.contains("verify token")
        || lower.contains("sign token")
        || lower.contains("token claims")
        || lower.contains("token expiry")
        || lower.contains("token expired")
        || lower.contains("eyj") // JWT header always starts with base64("{"alg")
        || lower.contains("hs256")
        || lower.contains("hs384")
        || lower.contains("hs512")
        || (lower.contains("token") && lower.contains("verify"))
        || (lower.contains("token") && lower.contains("sign"))
}

pub fn needs_k8s_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "kubernetes",
            "kubectl",
            "k8s",
            "kubernetes manifest",
            "k8s manifest",
            "kubernetes yaml",
            "k8s yaml",
            "deployment yaml",
            "pod spec",
            "kubernetes deployment",
            "kubernetes service",
            "kubernetes ingress",
            "kubernetes pod",
            "kubernetes configmap",
            "kubernetes statefulset",
            "kubernetes daemonset",
            "kubernetes job",
            "kubernetes cronjob",
            "kind: deployment",
            "kind: service",
            "kind: pod",
            "apiversion: apps/v1",
            "apiversion: v1",
            "livenessProbe",
            "readinessprobe",
            "resource limits",
            "kubernetes resource",
            "validate k8s",
            "validate kubernetes",
        ],
    )
}

/// Returns true when the user wants to parse, format, or convert XML — steer toward `xml_tools`.
pub fn needs_xml_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("xml")
        || lower.contains("parse xml")
        || lower.contains("format xml")
        || lower.contains("validate xml")
        || lower.contains("maven pom")
        || lower.contains("pom.xml")
        || lower.contains("android manifest")
        || lower.contains("spring config")
        || lower.contains("soap")
        || lower.contains("rss feed")
        || lower.contains("atom feed")
        || lower.contains("svg file")
        || lower.contains("xhtml")
        || lower.contains("xml to json")
        || lower.contains("convert xml")
        || lower.contains("xml element")
        || lower.contains("xml attribute")
        || lower.contains("<project>")
        || lower.contains("<?xml")
}

/// Returns true when the user wants to inspect a zip or archive — steer toward `archive_tools`.
pub fn needs_archive_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("zip file")
        || lower.contains("zip archive")
        || lower.contains("unzip")
        || lower.contains("extract zip")
        || lower.contains("list zip")
        || lower.contains("inspect zip")
        || lower.contains("inside the zip")
        || lower.contains("contents of the zip")
        || lower.contains("open zip")
        || lower.contains(".zip")
        || lower.contains(".jar")
        || lower.contains(".war")
        || lower.contains(".ear")
        || lower.contains(".whl")
        || lower.contains(".vsix")
        || lower.contains(".apk")
        || lower.contains("archive contents")
        || lower.contains("list archive")
        || lower.contains("what's in the archive")
        || lower.contains("peek inside")
        || (lower.contains("archive") && lower.contains("list"))
        || (lower.contains("archive") && lower.contains("extract"))
}

/// Returns true when the user wants to inspect or query a SQLite database — steer toward `sqlite_tools`.
pub fn needs_sqlite_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("sqlite")
        || lower.contains(".sqlite")
        || lower.contains(".db file")
        || lower.contains("sqlite3")
        || lower.contains("sql file")
        || lower.contains("query the database")
        || lower.contains("query database")
        || lower.contains("inspect the db")
        || lower.contains("what tables")
        || lower.contains("list tables")
        || lower.contains("show tables")
        || lower.contains("database schema")
        || lower.contains("db schema")
        || lower.contains("export table")
        || lower.contains("export csv")
        || lower.contains("table schema")
        || (lower.contains("database") && lower.contains("query"))
        || (lower.contains(".db")
            && (lower.contains("open") || lower.contains("read") || lower.contains("inspect")))
}

/// Returns true when the user wants to work with markdown content — steer toward `markdown_tools`.
pub fn needs_markdown_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("table of contents")
        || lower.contains("toc from")
        || lower.contains("generate toc")
        || lower.contains("markdown stats")
        || lower.contains("markdown statistics")
        || lower.contains("count words in")
        || lower.contains("word count in")
        || lower.contains("extract headings")
        || lower.contains("extract links")
        || lower.contains("extract code blocks")
        || lower.contains("links in the markdown")
        || lower.contains("links in this markdown")
        || lower.contains("markdown to html")
        || lower.contains("convert markdown to html")
        || lower.contains("render markdown")
        || lower.contains("strip markdown")
        || lower.contains("remove markdown")
        || lower.contains("plain text from markdown")
        || lower.contains("reading time")
        || (lower.contains("markdown") && lower.contains("heading"))
        || (lower.contains("markdown") && lower.contains("link"))
        || (lower.contains(".md file")
            && (lower.contains("parse") || lower.contains("analyze") || lower.contains("inspect")))
}

/// Returns true when the user wants URL parsing, building, or manipulation — steer toward `url_tools`.
pub fn needs_url_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("parse this url")
        || lower.contains("parse the url")
        || lower.contains("parse url")
        || lower.contains("decode url")
        || lower.contains("encode url")
        || lower.contains("url encode")
        || lower.contains("url decode")
        || lower.contains("percent encode")
        || lower.contains("percent decode")
        || lower.contains("query params")
        || lower.contains("query parameters")
        || lower.contains("query string")
        || lower.contains("url params")
        || lower.contains("build a url")
        || lower.contains("build url")
        || lower.contains("construct url")
        || lower.contains("normalize url")
        || lower.contains("validate url")
        || lower.contains("is this a valid url")
        || lower.contains("extract query")
        || lower.contains("add param")
        || lower.contains("remove param")
        || (lower.contains("url") && lower.contains("fragment"))
        || (lower.contains("url") && lower.contains("scheme"))
        || (lower.contains("url") && lower.contains("hostname"))
}

/// Returns true when the user wants line-based text processing — steer toward `line_tools`.
pub fn needs_line_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("grep for")
        || lower.contains("filter lines")
        || lower.contains("lines matching")
        || lower.contains("lines containing")
        || lower.contains("first 10 lines")
        || lower.contains("last 10 lines")
        || lower.contains("first n lines")
        || lower.contains("last n lines")
        || lower.contains("head of the file")
        || lower.contains("tail of the file")
        || lower.contains("sort these lines")
        || lower.contains("sort the lines")
        || lower.contains("sort lines")
        || lower.contains("unique lines")
        || lower.contains("deduplicate lines")
        || lower.contains("dedup lines")
        || lower.contains("count lines")
        || lower.contains("line count")
        || lower.contains("number the lines")
        || lower.contains("add line numbers")
        || lower.contains("join lines")
        || lower.contains("replace in text")
        || lower.contains("cut column")
        || lower.contains("cut field")
        || lower.contains("extract column")
        || lower.contains("slice lines")
        || lower.contains("lines from")
        || (lower.contains("text") && lower.contains("replace all"))
        || (lower.contains("file") && lower.contains("grep"))
}

/// Returns true when the user wants path manipulation — steer toward `path_tools`.
pub fn needs_path_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("parse this path")
        || lower.contains("parse the path")
        || lower.contains("path components")
        || lower.contains("join path")
        || lower.contains("join these paths")
        || lower.contains("normalize path")
        || lower.contains("relative path")
        || lower.contains("path relative to")
        || lower.contains("basename of")
        || lower.contains("dirname of")
        || lower.contains("file extension")
        || lower.contains("filename without extension")
        || lower.contains("stem of")
        || lower.contains("path stem")
        || lower.contains("is absolute")
        || lower.contains("absolute path")
        || lower.contains("is relative")
        || lower.contains("path manipulation")
        || lower.contains("split path")
        || lower.contains("path separator")
        || (lower.contains("change") && lower.contains("extension"))
        || (lower.contains("replace") && lower.contains("extension"))
}

/// Returns true when the user wants ASCII/markdown table formatting — steer toward `table_tools`.
pub fn needs_table_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("format as table")
        || lower.contains("format as a table")
        || lower.contains("ascii table")
        || lower.contains("format in columns")
        || lower.contains("align columns")
        || (lower.contains("align") && lower.contains("column"))
        || lower.contains("tabular format")
        || lower.contains("tabular view")
        || lower.contains("table view")
        || lower.contains("as a markdown table")
        || lower.contains("to markdown table")
        || lower.contains("markdown table")
        || lower.contains("pretty print table")
        || lower.contains("pretty-print table")
        || lower.contains("bordered table")
        || lower.contains("box table")
        || lower.contains("display as table")
        || lower.contains("show as table")
        || lower.contains("render as table")
        || lower.contains("format this data as a table")
        || lower.contains("transpose the table")
        || lower.contains("transpose rows")
        || (lower.contains("table") && lower.contains("from csv"))
        || (lower.contains("table") && lower.contains("from json"))
}

/// Returns true when the user wants hex dump, binary analysis, or encoding — steer toward `hex_tools`.
pub fn needs_hex_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("hex dump")
        || lower.contains("hexdump")
        || lower.contains("xxd")
        || lower.contains("hex encode")
        || lower.contains("hex decode")
        || lower.contains("decode hex")
        || lower.contains("encode hex")
        || lower.contains("to hex")
        || lower.contains("from hex")
        || lower.contains("hex string")
        || lower.contains("binary file")
        || lower.contains("magic bytes")
        || lower.contains("file signature")
        || lower.contains("file type detection")
        || lower.contains("extract strings")
        || lower.contains("strings from binary")
        || lower.contains("byte frequency")
        || lower.contains("shannon entropy")
        || lower.contains("entropy of")
        || (lower.contains("analyze") && lower.contains("binary"))
        || (lower.contains("inspect") && lower.contains("binary"))
}

/// Returns true when the user wants to read or query an INI/config file — steer toward `ini_tools`.
pub fn needs_ini_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains(".ini")
        || lower.contains(".cfg")
        || lower.contains(".conf")
        || lower.contains("ini file")
        || lower.contains("ini config")
        || lower.contains("config file")
        || lower.contains("parse ini")
        || lower.contains("read ini")
        || lower.contains("ini section")
        || lower.contains("ini key")
        || lower.contains("get from config")
        || lower.contains("config section")
        || lower.contains("configuration file")
        || lower.contains("ini to json")
        || lower.contains("ini to toml")
        || lower.contains("validate ini")
        || lower.contains("windows registry")
        || (lower.contains("section") && lower.contains("key") && lower.contains("value"))
}

/// Returns true when the user wants to parse or convert a duration — steer toward `duration_tools`.
pub fn needs_duration_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("parse duration")
        || lower.contains("humanize duration")
        || lower.contains("humanize seconds")
        || lower.contains("duration in seconds")
        || lower.contains("convert duration")
        || lower.contains("add duration")
        || lower.contains("seconds to hours")
        || lower.contains("seconds to minutes")
        || lower.contains("hours to seconds")
        || lower.contains("minutes to seconds")
        || lower.contains("duration format")
        || lower.contains("time duration")
        || lower.contains("how many seconds in")
        || lower.contains("duration breakdown")
        || (lower.contains("1h") && lower.contains("30m"))
        || lower.contains("pt1h")
        || lower.contains("pt2h")
        || (lower.contains("duration") && lower.contains("humanize"))
        || (lower.contains("humanize") && lower.contains("second"))
        || (lower.contains("seconds") && lower.contains("human readable"))
}

/// Returns true when the user wants to process ANSI escape codes — steer toward `ansi_tools`.
pub fn needs_ansi_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("ansi")
        || lower.contains("strip escape")
        || lower.contains("remove escape")
        || lower.contains("escape code")
        || lower.contains("terminal color")
        || lower.contains("colorize text")
        || lower.contains("color code")
        || lower.contains("visible length")
        || lower.contains("vt100")
        || lower.contains("sgr code")
        || (lower.contains("strip") && lower.contains("color"))
}

/// Returns true when the user wants to render a text template — steer toward `template_tools`.
pub fn needs_template_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("render template")
        || lower.contains("fill template")
        || lower.contains("template render")
        || lower.contains("variable substitution")
        || lower.contains("substitute variables")
        || lower.contains("placeholder")
        || lower.contains("mustache")
        || lower.contains("handlebars")
        || lower.contains("{{")
        || (lower.contains("template") && lower.contains("variable"))
        || (lower.contains("template") && lower.contains("fill"))
}

/// Returns true when the user wants to parse or convert a .env file — steer toward `dotenv_tools`.
pub fn needs_dotenv_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains(".env")
        || lower.contains("dotenv")
        || lower.contains("dot env")
        || lower.contains("merge env")
        || lower.contains("export env")
        || lower.contains("env to json")
        || lower.contains("env variables file")
        || (lower.contains("env file")
            && (lower.contains("parse")
                || lower.contains("read")
                || lower.contains("validate")
                || lower.contains("load")))
}

/// Returns true when the user wants Unicode character inspection — steer toward `char_tools`.
pub fn needs_char_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("unicode character")
        || lower.contains("codepoint")
        || lower.contains("code point")
        || lower.contains("unicode block")
        || lower.contains("unicode category")
        || lower.contains("escape unicode")
        || lower.contains("unescape unicode")
        || lower.contains("unicode escape")
        || lower.contains("is alphabetic")
        || lower.contains("is numeric")
        || lower.contains("char category")
        || lower.contains("character info")
        || lower.contains("what character")
        || lower.contains("what unicode")
        || (lower.contains("u+") && lower.contains("char"))
        || (lower.contains("\\u") && (lower.contains("escape") || lower.contains("unescape")))
}

/// Returns true when the user wants to parse an RSS or Atom feed — steer toward `rss_tools`.
pub fn needs_rss_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("rss feed")
        || lower.contains("atom feed")
        || lower.contains("parse feed")
        || lower.contains("parse rss")
        || lower.contains("parse atom")
        || lower.contains("rss entries")
        || lower.contains("feed entries")
        || lower.contains("news feed")
        || lower.contains("feed items")
        || lower.contains("<rss")
        || lower.contains("<feed")
        || lower.contains("podcast feed")
        || (lower.contains("feed") && lower.contains("xml"))
        || (lower.contains("rss")
            && (lower.contains("list") || lower.contains("read") || lower.contains("parse")))
}

/// Returns true when the user wants to store or retrieve key-value data — steer toward `keyval_tools`.
pub fn needs_keyval_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("key-value store")
        || lower.contains("keyvalue store")
        || lower.contains("key value store")
        || lower.contains("kv store")
        || lower.contains("store this value")
        || lower.contains("save this value")
        || lower.contains("remember this value")
        || lower.contains("retrieve a value")
        || lower.contains("keyval")
        || lower.contains("hematite kv")
        || (lower.contains("set key") && lower.contains("value"))
        || (lower.contains("get key") && lower.contains("store"))
        || (lower.contains("store") && lower.contains("key") && lower.contains("value"))
}

/// Returns true when the user wants statistical analysis on numbers — steer toward `stat_tools`.
pub fn needs_stat_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("descriptive statistics")
        || lower.contains("statistical summary")
        || lower.contains("mean and median")
        || lower.contains("mean median")
        || lower.contains("standard deviation")
        || lower.contains("outlier detection")
        || lower.contains("find outliers")
        || lower.contains("detect outliers")
        || lower.contains("percentile")
        || lower.contains("z-score")
        || lower.contains("zscore")
        || lower.contains("histogram of")
        || lower.contains("frequency distribution")
        || lower.contains("mode of these")
        || lower.contains("correlate these")
        || lower.contains("pearson correlation")
        || (lower.contains("statistics") && lower.contains("numbers"))
        || (lower.contains("stats") && lower.contains("array"))
        || (lower.contains("describe") && lower.contains("dataset"))
        || (lower.contains("mean") && lower.contains("stddev"))
        || (lower.contains("mean") && lower.contains("std dev"))
}

/// Returns true when the user wants to look up ports, services, or IP protocol numbers
/// — steer toward `net_lookup_tools`.
pub fn needs_nginx_conf_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "nginx.conf",
            "nginx config",
            "nginx configuration",
            "nginx server block",
            "nginx location",
            "parse nginx",
            "explain nginx",
            "validate nginx",
            "nginx upstream",
            "nginx proxy_pass",
            "nginx vhost",
            "nginx virtual host",
            "nginx sites-available",
            "nginx sites-enabled",
        ],
    )
}

pub fn needs_openapi_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "openapi",
            "open api",
            "swagger",
            "swagger spec",
            "api spec",
            "oas3",
            "oas 3",
            "swagger.yaml",
            "swagger.json",
            "openapi.yaml",
            "openapi.json",
            "api endpoints",
            "parse openapi",
            "validate openapi",
            "api schemas",
            "api definitions",
        ],
    )
}

pub fn needs_net_lookup_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("what port")
        || lower.contains("which port")
        || lower.contains("port number")
        || lower.contains("port lookup")
        || lower.contains("well-known port")
        || lower.contains("well known port")
        || lower.contains("service port")
        || lower.contains("ip protocol number")
        || lower.contains("protocol number")
        || lower.contains("iana protocol")
        || lower.contains("what service runs on")
        || lower.contains("what runs on port")
        || lower.contains("which service uses port")
        || (lower.contains("port") && lower.contains("service name"))
        || (lower.contains("lookup") && lower.contains("port"))
        || (lower.contains("look up") && lower.contains("port"))
        || (lower.contains("net_lookup") || lower.contains("net lookup"))
}

/// Returns true when the user wants to parse or convert data sizes — steer toward `size_tools`.
pub fn needs_size_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("convert bytes")
        || lower.contains("bytes to mb")
        || lower.contains("bytes to gb")
        || lower.contains("mb to gb")
        || lower.contains("gb to tb")
        || lower.contains("gib to gb")
        || lower.contains("mib to mb")
        || lower.contains("file size")
        || lower.contains("memory size")
        || lower.contains("parse size")
        || lower.contains("size_tools")
        || lower.contains("transfer time")
        || lower.contains("download time")
        || lower.contains("how long to download")
        || lower.contains("bandwidth calculation")
        || lower.contains("how many bytes")
        || lower.contains("kibibyte")
        || lower.contains("mebibyte")
        || lower.contains("gibibyte")
        || (lower.contains("size") && lower.contains("convert"))
        || (lower.contains("gb") && lower.contains("mb") && lower.contains("convert"))
        || (lower.contains("mbps") && (lower.contains("time") || lower.contains("download")))
}

/// Returns true when the user wants to validate a common data format — steer toward `validate_tools`.
pub fn needs_validate_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("validate email")
        || lower.contains("valid email")
        || lower.contains("is this a valid email")
        || lower.contains("validate ip")
        || lower.contains("valid ipv4")
        || lower.contains("valid ipv6")
        || lower.contains("validate mac")
        || lower.contains("valid mac address")
        || lower.contains("validate url")
        || lower.contains("valid url")
        || lower.contains("validate uuid")
        || lower.contains("valid uuid")
        || lower.contains("validate credit card")
        || lower.contains("luhn check")
        || lower.contains("validate isbn")
        || lower.contains("isbn check")
        || lower.contains("validate phone")
        || lower.contains("validate semver")
        || lower.contains("validate hex color")
        || lower.contains("validate cidr")
        || lower.contains("validate_tools")
        || (lower.contains("validate") && lower.contains("format"))
        || (lower.contains("is")
            && lower.contains("valid")
            && (lower.contains("email")
                || lower.contains("ip")
                || lower.contains("url")
                || lower.contains("uuid")
                || lower.contains("isbn")
                || lower.contains("mac")))
}

/// Returns true when the user wants financial calculations — steer toward `money_tools`.
pub fn needs_money_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("compound interest")
        || lower.contains("loan payment")
        || lower.contains("monthly payment")
        || lower.contains("mortgage payment")
        || lower.contains("apr to apy")
        || lower.contains("apy from apr")
        || lower.contains("annual percentage")
        || lower.contains("percent discount")
        || lower.contains("percentage discount")
        || lower.contains("calculate discount")
        || lower.contains("percent of")
        || lower.contains("tip calculator")
        || lower.contains("split bill")
        || lower.contains("split the bill")
        || lower.contains("format currency")
        || lower.contains("format money")
        || lower.contains("money_tools")
        || lower.contains("money tools")
        || (lower.contains("interest") && lower.contains("rate") && lower.contains("years"))
        || (lower.contains("loan") && lower.contains("monthly"))
        || (lower.contains("tip") && lower.contains("restaurant"))
        || (lower.contains("bill") && lower.contains("split") && lower.contains("people"))
}

/// Returns true when the user wants extended financial analysis — steer toward `financial_tools`.
pub fn needs_financial_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("amortization")
        || lower.contains("amortize")
        || lower.contains("amortisation")
        || lower.contains("depreciation")
        || lower.contains("straight line depreciation")
        || lower.contains("double declining")
        || lower.contains("sum of years")
        || lower.contains("macrs")
        || lower.contains("roi calculation")
        || lower.contains("return on investment")
        || lower.contains("break-even")
        || lower.contains("breakeven")
        || lower.contains("break even analysis")
        || lower.contains("npv calculation")
        || lower.contains("net present value")
        || lower.contains("internal rate of return")
        || lower.contains(" irr ")
        || lower.ends_with(" irr")
        || lower.starts_with("irr ")
        || lower.contains("cagr")
        || lower.contains("compound annual growth")
        || lower.contains("savings goal")
        || lower.contains("savings planner")
        || lower.contains("savings plan")
        || lower.contains("payback period")
        || lower.contains("cash flow analysis")
        || lower.contains("cashflow analysis")
        || lower.contains("financial_tools")
        || (lower.contains("depreciate") && lower.contains("asset"))
        || (lower.contains("npv") && lower.contains("cashflow"))
        || (lower.contains("loan") && lower.contains("amortiz"))
}

pub fn needs_token_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "estimate tokens",
            "token count",
            "how many tokens",
            "token budget",
            "context window fill",
            "fits in context window",
            "truncate to tokens",
            "token estimate",
            "tokens in this text",
            "token cost",
            "context fill",
            "llm token",
        ],
    )
}

pub fn needs_mime_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "mime type",
            "mimetype",
            "content-type header",
            "media type for",
            "file extension for mime",
            "look up mime",
            "mime for .",
            "which mime",
            "content type for .",
            "application/pdf",
            "image/png",
            "image/jpeg",
            "text/html",
            "text/plain",
            "audio/mpeg",
            "video/mp4",
        ],
    )
}

pub fn needs_robots_txt_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "robots.txt",
            "robots txt",
            "parse robots",
            "check robots",
            "validate robots",
            "user-agent block",
            "disallow rule",
            "crawl-delay",
            "is this url allowed",
            "is this path blocked",
            "can googlebot crawl",
            "crawl rules",
        ],
    )
}

pub fn needs_sitemap_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "sitemap.xml",
            "sitemap xml",
            "parse sitemap",
            "search sitemap",
            "list sitemap",
            "sitemap stats",
            "sitemap index",
            "urlset",
            "sitemap urls",
            "how many urls in sitemap",
        ],
    )
}

pub fn needs_make_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "makefile",
            "make target",
            "make targets",
            "list make",
            "make deps",
            "make variables",
            "explain make",
            "parse makefile",
            "makefile target",
            "makefile variable",
            "make recipe",
            "make rule",
            "makefile deps",
            "gnumake",
        ],
    )
}

pub fn needs_changelog_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "changelog",
            "release notes",
            "parse changelog",
            "read changelog",
            "latest release",
            "changelog version",
            "what changed in version",
            "changelog.md",
            "keep a changelog",
            "release history",
        ],
    )
}

pub fn needs_github_actions_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "github actions",
            "github workflow",
            ".github/workflows",
            "actions workflow",
            "ci workflow yaml",
            "workflow yaml",
            "workflow triggers",
            "workflow jobs",
            "workflow steps",
            "parse workflow",
            "validate workflow",
            "actions on:",
            "runs-on:",
            "uses: actions/",
            "workflow_dispatch",
            "on: push",
            "on: pull_request",
        ],
    )
}

pub fn needs_gitignore_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".gitignore",
            "gitignore",
            "check gitignore",
            "is this file ignored",
            "git ignore pattern",
            "ignored by git",
            "generate gitignore",
            "gitignore for",
            "gitignore pattern",
            "explain gitignore",
            "parse gitignore",
            "validate gitignore",
        ],
    )
}

pub fn needs_license_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "software license",
            "open source license",
            "license info",
            "license comparison",
            "compare licenses",
            "mit license",
            "apache license",
            "gpl license",
            "detect license",
            "identify license",
            "what license",
            "license file",
            "spdx",
            "copyleft",
            "permissive license",
            "license for my project",
            "which license",
        ],
    )
}

pub fn needs_ssh_config_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ssh config",
            "ssh_config",
            "~/.ssh/config",
            "ssh configuration",
            "explain ssh config",
            "parse ssh config",
            "validate ssh config",
            "ssh host alias",
            "ssh host block",
            "proxyjump",
            "identityfile",
            "stricthostkeychecking",
            "ssh identityfile",
            "ssh host options",
        ],
    )
}

pub fn needs_systemd_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "systemd unit",
            "systemd service",
            "systemd timer",
            "systemd socket",
            ".service file",
            ".timer file",
            ".socket file",
            "unit file",
            "validate unit",
            "parse unit file",
            "execstart",
            "wantedby=",
            "oncalendar",
            "onbootsec",
            "systemctl enable",
            "systemd hardening",
            "privatetmp",
            "nonewprivileges",
            "protectsystem",
        ],
    )
}

pub fn needs_log_parse_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "parse log",
            "parse logs",
            "parse this log",
            "log file parse",
            "parse json logs",
            "parse syslog",
            "parse apache log",
            "parse nginx log",
            "log line parse",
            "filter log",
            "log stats",
            "log format detect",
            "structured log",
            "key=value log",
            "access log parse",
        ],
    )
}

pub fn needs_csp_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "content security policy",
            "csp header",
            "content-security-policy",
            "parse csp",
            "explain csp",
            "validate csp",
            "build csp",
            "csp directive",
            "unsafe-inline",
            "unsafe-eval",
            "default-src",
            "script-src",
            "frame-ancestors",
        ],
    )
}

pub fn needs_http_status_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "http status code",
            "http status",
            "what does http",
            "http error code",
            "status code meaning",
            "what is a 404",
            "what is a 500",
            "what is a 200",
            "what is a 422",
            "what is a 429",
            "http 4xx",
            "http 5xx",
            "http 2xx",
            "http 3xx",
            "list http codes",
            "search http codes",
        ],
    )
}

pub fn needs_http_parse_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "parse http",
            "parse this http",
            "parse this request",
            "parse this response",
            "parse http request",
            "parse http response",
            "parse raw http",
            "http message",
            "inspect http headers",
            "analyze http headers",
            "decode http",
            "read http headers",
            "http cookies",
            "parse cookie header",
            "set-cookie header",
            "http auth header",
            "authorization header",
            "www-authenticate",
            "basic auth header",
            "bearer token header",
            "http request headers",
            "http response headers",
            "raw http message",
            "raw http request",
            "raw http response",
        ],
    ) || (lower.contains("raw") && lower.contains("http"))
}

pub fn needs_jq_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "jq query",
            "jq filter",
            "jq expression",
            "jq path",
            "run jq",
            "use jq",
            "json path",
            "json query",
            "json filter",
            "query json",
            "filter json",
            "navigate json",
            "json field access",
            "extract from json",
            "extract json",
            "json field",
            "json array filter",
            "flatten json",
            "flatten nested json",
            "json flatten",
            "map json array",
            "select from json",
            "json keys at path",
            "json values at",
            "count json",
            "json type check",
        ],
    )
}

pub fn needs_glob_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "glob pattern",
            "test glob",
            "glob match",
            "glob filter",
            "glob to regex",
            "convert glob",
            "explain glob",
            "gitignore pattern",
            "file pattern match",
            "wildcard pattern",
            "does this glob",
            "filter paths with",
            "match files with",
        ],
    )
}

pub fn needs_graph_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "graph algorithm",
            "bfs traversal",
            "breadth first search",
            "dfs traversal",
            "depth first search",
            "shortest path",
            "dijkstra",
            "topological sort",
            "topo sort",
            "detect cycle",
            "graph cycle",
            "connected components",
            "graph components",
            "graph nodes",
            "graph edges",
            "strongly connected",
            "scc algorithm",
            "graph theory",
            "traverse graph",
            "path in graph",
        ],
    )
}

pub fn needs_matrix_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "matrix multiply",
            "multiply matrices",
            "matrix multiplication",
            "matrix transpose",
            "transpose matrix",
            "matrix determinant",
            "determinant of",
            "matrix inverse",
            "inverse matrix",
            "invert matrix",
            "solve linear",
            "linear system",
            "gaussian elimination",
            "matrix rank",
            "rank of matrix",
            "matrix info",
            "matrix stats",
            "is matrix singular",
            "matrix operations",
            "linear algebra",
        ],
    )
}

pub fn needs_sql_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "sql query",
            "sql file",
            "sql statement",
            "sql schema",
            "create table",
            "select statement",
            "explain this sql",
            "parse sql",
            "validate sql",
            "analyze sql",
            "sql joins",
            "sql ddl",
            "sql dml",
            "database schema sql",
            "check this query",
            "review this query",
        ],
    ) || (lower.contains(".sql") && !lower.contains("nosql"))
        || lower.contains("create table ")
        || lower.contains("select * from")
        || lower.contains("insert into ")
}

pub fn needs_proto_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "proto file",
            ".proto",
            "protobuf",
            "protocol buffer",
            "grpc",
            "proto schema",
            "proto message",
            "proto service",
            "proto rpc",
            "validate proto",
            "parse proto",
            "review proto",
            "proto definition",
            "protobuf message",
            "rpc method",
        ],
    )
}

pub fn needs_terraform_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "terraform",
            ".tf file",
            "hcl file",
            "main.tf",
            "variables.tf",
            "outputs.tf",
            "terraform resource",
            "terraform variable",
            "terraform output",
            "terraform module",
            "terraform provider",
            "tf resource",
            "validate terraform",
            "review terraform",
            "parse terraform",
            "infrastructure as code",
            "iac file",
        ],
    ) || lower.contains(".tf\"")
        || lower.contains(".tf'")
        || lower.ends_with(".tf")
}

pub fn needs_graphviz_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "dot language",
            "graphviz",
            "generate dot",
            "dot graph",
            "digraph",
            "dot file",
            ".dot file",
            "generate flowchart dot",
            "generate dot diagram",
            "dot format",
            "dot syntax",
            "render with dot",
            "graphviz flowchart",
            "graphviz tree",
        ],
    )
}

pub fn needs_mermaid_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "mermaid diagram",
            "mermaid chart",
            "mermaid flowchart",
            "mermaid sequence",
            "mermaid class diagram",
            "mermaid gantt",
            "mermaid er",
            "mermaid pie",
            "mermaid.js",
            "generate mermaid",
            "sequence diagram",
            "class diagram",
            "er diagram",
            "gantt chart",
            "mermaid syntax",
            "mermaid code",
        ],
    )
}

pub fn needs_graphql_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "graphql",
            "gql",
            ".graphql",
            "graphql schema",
            "graphql query",
            "graphql mutation",
            "graphql type",
            "graphql fragment",
            "graphql subscription",
            "introspection",
            "parse graphql",
            "validate graphql",
            "inspect graphql",
            "review graphql",
            "graphql operation",
            "type Query",
            "type Mutation",
        ],
    )
}

pub fn needs_sql_migrate_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "sql migration",
            "sql migrate",
            "migration file",
            "migration script",
            "db migration",
            "database migration",
            "analyze migration",
            "review migration",
            "validate migration",
            "migration risk",
            "risky migration",
            "safe migration",
            "migration ops",
            "schema migration",
            "alembic",
            "flyway",
            "liquibase",
            "rails migration",
            "django migration sql",
            ".sql migration",
        ],
    )
}

pub fn needs_pem_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pem file",
            ".pem",
            "tls cert",
            "ssl cert",
            "x.509",
            "x509",
            "certificate",
            "certificates",
            "cert chain",
            "certificate chain",
            "cert expire",
            "cert expir",
            "cert valid",
            "inspect cert",
            "validate cert",
            "decode cert",
            "parse cert",
            "pem block",
            "private key pem",
            "public key pem",
            "-----begin",
            "san extension",
            "subject alternative name",
        ],
    )
}

pub fn needs_env_schema_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".env.example",
            "env example",
            "env schema",
            "env template",
            "validate env",
            "validate .env",
            "missing env",
            "missing keys in env",
            "env against",
            "compare env",
            "env diff against",
            "required env",
            "env vars missing",
            "env variables missing",
            "env completeness",
            "env compliance",
            "check env",
        ],
    )
}

pub fn needs_package_json_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "package.json",
            "npm scripts",
            "node dependencies",
            "npm deps",
            "npm package",
            "parse package.json",
            "validate package.json",
            "review package.json",
            "package scripts",
            "node package",
            "devdependencies",
            "peerdependencies",
            "npm version",
            "what scripts are in",
            "list npm scripts",
        ],
    )
}

pub fn needs_base_tools(user_input: &str) -> bool {
    contains_any(
        user_input,
        &[
            "base32",
            "base58",
            "base85",
            "base16 encode",
            "b32 encode",
            "b58 encode",
            "z85",
            "ascii85",
            "encode base",
            "decode base",
            "identify encoding",
            "guess encoding",
            "bitcoin base58",
            "ipfs base58",
            "rfc 4648",
        ],
    )
}

pub fn needs_lock_file_tools(user_input: &str) -> bool {
    contains_any(
        user_input,
        &[
            "cargo.lock",
            "package-lock.json",
            "yarn.lock",
            "poetry.lock",
            "lock file",
            "lockfile",
            "lock file analysis",
            "lock file packages",
            "analyze lock",
            "parse lock",
            "dependency lock",
            "duplicate dependencies",
            "duplicate packages",
            "dedupe",
            "lock file duplicates",
            "packages in lock",
            "what packages are locked",
        ],
    )
}

pub fn needs_fraction_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "fraction",
            "numerator",
            "denominator",
            "simplify fraction",
            "reduce fraction",
            "add fractions",
            "subtract fractions",
            "multiply fractions",
            "divide fractions",
            "decimal to fraction",
            "fraction to decimal",
            "mixed number",
            "harmonic series",
            "egyptian fraction",
            "farey sequence",
            "rational number",
            "lowest terms",
        ],
    )
}

pub fn needs_number_theory_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "prime factorization",
            "prime factors",
            "is prime",
            "list primes",
            "nth prime",
            "sieve of eratosthenes",
            "euler totient",
            "euler's totient",
            "phi(",
            "modular inverse",
            "modinv",
            "modpow",
            "modular exponentiation",
            "collatz",
            "fibonacci sequence",
            "fibonacci number",
            "perfect number",
            "abundant number",
            "deficient number",
            "number theory",
            "divisors of",
            "divisor sum",
            "bezout",
            "bézout",
            "coprime",
        ],
    ) || (lower.contains("prime")
        && (lower.contains("is ") || lower.contains("check") || lower.contains("test")))
}

pub fn needs_cipher_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "caesar cipher",
            "vigenere cipher",
            "vigenere",
            "vigenère cipher",
            "vigenère",
            "rot13",
            "atbash",
            "rail fence",
            "classical cipher",
            "encode cipher",
            "decode cipher",
            "encrypt caesar",
            "decrypt caesar",
            "cipher text",
            "frequency analysis",
            "index of coincidence",
            "cipher break",
            "monoalphabetic",
            "polyalphabetic",
            "encode with key",
            "decrypt vigenere",
        ],
    )
}

pub fn needs_nato_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "nato alphabet",
            "nato phonetic",
            "phonetic alphabet",
            "spell out letters",
            "morse code",
            "morse",
            "encode morse",
            "decode morse",
            "dit dah",
            "alpha bravo charlie",
            "hotel india juliet",
            "foxtrot golf",
        ],
    )
}

pub fn needs_geo_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "haversine",
            "great circle",
            "gps coordinates",
            "latitude longitude",
            "lat lng",
            "lat lon",
            "degrees minutes seconds",
            "dms coordinates",
            "decimal degrees",
            "geographic distance",
            "distance between coordinates",
            "bearing between",
            "compass bearing",
            "geographic midpoint",
            "bounding box coordinates",
            "destination coordinate",
            "navigation bearing",
            "coordinate conversion",
            "geo distance",
        ],
    )
}

pub fn needs_data_gen_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "lorem ipsum",
            "generate lorem",
            "fake names",
            "random names",
            "test data generation",
            "generate test data",
            "mock data",
            "dummy data",
            "sample names",
            "random emails",
            "fake email",
            "generate email",
            "sequential ids",
            "generate ids",
            "fake uuid",
            "random dates",
            "generate dates",
            "test fixture",
            "placeholder data",
            "filler text",
        ],
    )
}

pub fn needs_unit_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "convert meters",
            "convert kilometres",
            "convert miles",
            "convert kg",
            "convert pounds",
            "convert celsius",
            "convert fahrenheit",
            "convert kelvin",
            "unit conversion",
            "convert units",
            "metres to feet",
            "feet to metres",
            "km to miles",
            "miles to km",
            "kg to pounds",
            "pounds to kg",
            "fahrenheit to celsius",
            "celsius to fahrenheit",
            "temperature in celsius",
            "temperature in fahrenheit",
            "convert knots",
            "convert mph",
            "convert kph",
            "litres to gallons",
            "gallons to litres",
            "convert joules",
            "convert calories",
            "convert watts",
            "convert horsepower",
            "convert psi",
            "convert bar",
            "convert hectares",
            "convert acres",
            "convert square",
            "megahertz to gigahertz",
            "hertz to",
            "convert hertz",
            "convert frequency",
            "convert pressure",
            "list units",
            "what units",
            "available units",
        ],
    )
}

pub fn needs_geometry_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "area of a circle",
            "area of a rectangle",
            "area of a triangle",
            "area of a square",
            "area of a trapezoid",
            "area of an ellipse",
            "area of a polygon",
            "volume of a sphere",
            "volume of a cylinder",
            "volume of a cone",
            "volume of a cube",
            "surface area of",
            "perimeter of a",
            "circumference of",
            "geometry calculation",
            "geometric shape",
            "triangle sides",
            "solve triangle",
            "triangle angles",
            "right triangle",
            "hypotenuse",
            "inradius",
            "circumradius",
            "circle radius",
            "circle area",
            "circle circumference",
            "arc length",
            "sector area",
            "chord length",
            "bounding box",
            "calculate area",
            "calculate volume",
            "calculate perimeter",
        ],
    )
}

pub fn needs_bio_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "dna sequence",
            "rna sequence",
            "nucleotide sequence",
            "protein sequence",
            "reverse complement",
            "dna complement",
            "transcribe dna",
            "translate dna",
            "translate mrna",
            "translate rna",
            "gc content",
            "open reading frame",
            "find orfs",
            "codon usage",
            "codon table",
            "parse fasta",
            "fasta file",
            "fasta sequence",
            "amino acid sequence",
            "atgc",
            "nucleotides",
            "bioinformatics",
            "genetic sequence",
            "gene sequence",
            "mrna sequence",
            "dna to rna",
            "rna to protein",
        ],
    )
}

pub fn needs_gpu_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "vram estimate",
            "model vram",
            "vram for model",
            "how much vram",
            "vram needed",
            "vram requirement",
            "gpu vram",
            "gpu memory",
            "llm vram",
            "model memory",
            "quantization vram",
            "batch size vram",
            "fit in vram",
            "fit in gpu",
            "gpu specs",
            "gpu specification",
            "rtx 4070 specs",
            "rtx 4090 specs",
            "rtx 3090 specs",
            "nvidia gpu spec",
            "parse nvidia-smi",
            "nvidia-smi output",
            "vram budget",
            "gpu budget",
            "llm quantization",
            "q4 model size",
            "q8 model size",
            "fp16 model size",
            "gguf size",
            "gguf vram",
            "parameter count vram",
            "7b vram",
            "13b vram",
            "70b vram",
        ],
    )
}

pub fn needs_checksum_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "crc32",
            "crc-32",
            "crc16",
            "crc-16",
            "crc8",
            "crc-8",
            "adler32",
            "adler-32",
            "fletcher",
            "checksum",
            "compute checksum",
            "calculate checksum",
            "cyclic redundancy",
            "error detection code",
            "data integrity check",
        ],
    )
}

pub fn needs_id_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ulid",
            "generate ulid",
            "nanoid",
            "generate nanoid",
            "nano id",
            "snowflake id",
            "snowflake identifier",
            "time-sortable id",
            "sortable unique id",
            "generate id",
            "decode ulid",
            "decode snowflake",
            "twitter snowflake",
            "discord id",
        ],
    )
}

pub fn needs_binary_tools(user_input: &str) -> bool {
    contains_any(
        user_input,
        &[
            "bit manipulation",
            "bitfield",
            "bit field",
            "pack bits",
            "unpack bits",
            "flag bits",
            "bitmask",
            "bit mask",
            "bit operations",
            "bitwise ops",
            "popcount",
            "bit count",
            "gray code",
            "rotate bits",
            "bit shift",
            "set bit",
            "clear bit",
            "toggle bit",
            "bit packing",
            "binary flags",
            "binary field",
            "ieee 754 float",
            "bit decompose",
            "bit layout",
        ],
    )
}

pub fn needs_ascii_tools(user_input: &str) -> bool {
    contains_any(
        user_input,
        &[
            "ascii art",
            "ascii banner",
            "big text",
            "big letters",
            "ascii box",
            "box drawing",
            "draw box",
            "draw a box",
            "ascii table",
            "progress bar",
            "ascii bar",
            "ascii tree",
            "text tree",
            "directory tree",
            "tree diagram",
            "ascii progress",
            "ascii border",
            "ascii frame",
            "banner text",
            "terminal banner",
            "ascii chart",
        ],
    )
}

pub fn needs_time_zone_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "time zone",
            "timezone",
            "convert time",
            "time in ",
            "utc offset",
            "utc to ",
            "to utc",
            "gmt offset",
            "gmt to ",
            "to gmt",
            "what time is it in",
            "world clock",
            "local time",
            "time difference",
            "time conversion",
            "pst to est",
            "est to pst",
            "cst to est",
            "ist to utc",
            "list timezones",
            "list time zones",
            "dst offset",
            "daylight saving",
        ],
    )
}

pub fn needs_word_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "word frequency",
            "word count frequency",
            "most common words",
            "frequent words",
            "anagram",
            "soundex",
            "phonetic match",
            "sounds like",
            "palindrome",
            "syllable",
            "syllables",
            "syllable count",
            "flesch-kincaid",
            "flesch kincaid",
            "readability grade",
            "word analysis",
            "count syllables",
        ],
    )
}

pub fn needs_string_metric_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "levenshtein",
            "edit distance",
            "string distance",
            "damerau",
            "jaro winkler",
            "jaro-winkler",
            "jaro similarity",
            "hamming distance",
            "lcs ",
            "longest common subsequence",
            "string similarity",
            "fuzzy match",
            "fuzzy search",
            "string metric",
            "phonetic similarity",
            "approximate match",
            "string compare",
            "how similar are",
        ],
    )
}

pub fn needs_calc_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    // Avoid triggering on generic "calculate X" when a more specific tool applies
    // Focus on expression evaluation patterns
    contains_any(
        &lower,
        &[
            "evaluate expression",
            "eval expression",
            "calculate expression",
            "rpn calculator",
            "reverse polish",
            "math expression",
            "formula eval",
            "evaluate formula",
            "compute expression",
            "expression evaluator",
            "simple calculator",
            "quick calculation",
            "calc(",
            "factorial(",
            "sin(",
            "cos(",
            "tan(",
            "sqrt(",
            "log(",
            "variable expression",
            "sequence generator",
            "numeric sequence",
        ],
    )
}

pub fn needs_port_check(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "port check",
            "check port",
            "test port",
            "is port open",
            "port open",
            "port reachable",
            "can i connect to port",
            "tcp port test",
            "port 5432",
            "port 6379",
            "port 3306",
            "port 443",
            "port 80",
            "port 22",
            "port 3389",
            "port connectivity",
            "port accessible",
            "port closed",
            "port filtered",
            "is postgres up",
            "is redis up",
            "is mysql up",
            "service port",
        ],
    )
}

pub fn needs_scientific_compute(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "scientific compute",
            "scientific calculation",
            "physics calculation",
            "chemistry calculation",
            "physical constant",
            "boltzmann",
            "planck constant",
            "avogadro",
            "speed of light",
            "electron mass",
            "stefan-boltzmann",
            "ideal gas law",
            "kinetic energy formula",
            "potential energy",
            "wave equation",
            "ohm's law",
            "ohms law",
            "coulomb's law",
            "coulombs law",
            "periodic table compute",
            "atomic mass",
            "molar mass",
            "scientific notation compute",
        ],
    )
}

pub fn needs_template_gen(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "generate dockerfile",
            "create dockerfile",
            "dockerfile template",
            "scaffold dockerfile",
            "generate a makefile",
            "create a makefile",
            "makefile template",
            "generate ci",
            "generate github actions",
            "ci pipeline template",
            "github actions template",
            "project scaffold",
            "scaffold project",
            "scaffold a new",
            "new project template",
            "project template",
            "docker-compose template",
            "generate docker-compose",
            "generate .env.example",
            "env example template",
            "editorconfig template",
            "pre-commit config",
            "dependabot config",
            "codeowners template",
            "pr template",
            "pull request template",
            "issue template",
        ],
    )
}

pub fn needs_changelog_gen(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "generate changelog",
            "generate a changelog",
            "create changelog",
            "write changelog",
            "changelog from commits",
            "changelog from git",
            "changelog from git log",
            "generate release notes from",
            "create release notes from",
            "write release notes from",
            "commit history changelog",
            "git log to changelog",
            "auto-generate changelog",
            "conventional commits changelog",
        ],
    )
}

pub fn needs_dependency_audit(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "dependency audit",
            "audit dependencies",
            "audit deps",
            "check dependencies",
            "outdated dependencies",
            "outdated deps",
            "pinned versions",
            "wildcard version",
            "unpinned dep",
            "dependency versions",
            "cargo dependencies",
            "npm dependencies",
            "python dependencies",
            "requirements.txt",
            "pyproject.toml dep",
            "go.mod dep",
            "missing lock file",
            "lock file missing",
            "deprecated package",
            "scan dependencies",
        ],
    )
}

pub fn needs_env_diff(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "env diff",
            "diff env",
            "compare env",
            ".env diff",
            ".env compare",
            "compare .env",
            "env file diff",
            "environment diff",
            "diff environment",
            "env variables diff",
            "missing env vars",
            "env mismatch",
            "env changes",
            "env vs production",
            "env vs staging",
            "dotenv diff",
            "env added",
            "env removed",
            "env changed",
        ],
    )
}

pub fn needs_json_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "json query",
            "jq query",
            "query json",
            "parse json",
            "pretty print json",
            "format json",
            "json path",
            "json filter",
            "json get ",
            "json keys",
            "json sort",
            "json diff",
            "diff json",
            "json merge",
            "merge json",
            "json to csv",
            "json schema",
            "json validate",
            "validate json",
            "json stats",
            "json transform",
            "flatten json",
            "json pluck",
            "count json",
            "json unique",
        ],
    )
}

/// Returns true when the user wants to parse or analyze a DNS zone file — steer toward `dns_tools`.
pub fn needs_dns_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "zone file",
            "dns zone",
            "parse zone",
            "bind zone",
            "zone records",
            "dns records",
            "mx record",
            "a record dns",
            "aaaa record",
            "cname record",
            "txt record",
            "soa record",
            "ns record",
            "ptr record",
            "srv record",
            "caa record",
            "dkim record",
            "dmarc record",
            "spf record",
            "dns_tools",
            "validate zone",
            "explain dns",
            "named.conf",
            "db.example",
        ],
    ) || (lower.contains("dns") && lower.contains("zone"))
        || (lower.contains("spf") && lower.contains("txt"))
        || (lower.contains("dmarc") && lower.contains("policy"))
}

/// Returns true when the user wants to parse, validate, or analyze CSS — steer toward `css_tools`.
pub fn needs_css_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "parse css",
            "css file",
            "css selector",
            "css variables",
            "css custom properties",
            "css stats",
            "minify css",
            "css validation",
            "validate css",
            "css rule",
            "stylesheet",
            "css specificity",
            "css at-rule",
            "css media query",
            "css keyframes",
            "css_tools",
            "css duplicate",
            "css !important",
            "vendor prefix css",
            "css var(",
            ".css file",
        ],
    ) || (lower.contains("css") && lower.contains("minif"))
        || (lower.contains("css") && lower.contains("variable"))
        || (lower.contains("css") && lower.contains("parse"))
        || (lower.contains("css") && lower.contains("duplicate"))
        || (lower.contains("style") && lower.contains("css") && lower.contains("analyz"))
}

pub fn needs_code_metrics(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "code metrics",
            "lines of code",
            "line count",
            "loc count",
            "count lines of code",
            "comment density",
            "todo count",
            "fixme count",
            "code statistics",
            "code stats",
            "source code stats",
            "codebase size",
            "largest files",
            "test ratio",
            "code coverage proxy",
            "language breakdown",
            "file breakdown",
            "code complexity",
        ],
    )
}

/// Returns true when the user wants to parse or inspect a plist file — steer toward `plist_tools`.
pub fn needs_plist_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "plist",
            "info.plist",
            "cfbundle",
            "apple property list",
            "ios app metadata",
            "macos plist",
            "parse plist",
            "validate plist",
            "plist to json",
            "nstransport",
            "nsallowsarbitraryloads",
            "nsapptransportsecurity",
            "nsusagedescription",
            "minimum os version",
            "lsminimum",
        ],
    )
}

/// Returns true when the user wants to decode or inspect bencode or a .torrent file — steer toward `bencode_tools`.
pub fn needs_bencode_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "bencode",
            "bencoded",
            ".torrent",
            "torrent file",
            "parse torrent",
            "decode torrent",
            "torrent info",
            "torrent tracker",
            "torrent files",
            "files in torrent",
            "piece length",
            "announce list",
            "bittorrent",
            "magnet link metadata",
            "torrent metadata",
            "inspect torrent",
        ],
    )
}

/// Returns true when the user wants to analyze or simulate a C-style printf format string — steer toward `printf_tools`.
pub fn needs_printf_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "printf format",
            "printf string",
            "format specifier",
            "format string",
            "%d %s",
            "%s %d",
            "explain printf",
            "simulate printf",
            "validate printf",
            "printf specifier",
            "printf placeholder",
            "printf syntax",
            "c format string",
            "sprintf format",
            "fprintf format",
            "printf conversion",
            "convert printf",
            "printf to python",
            "printf to rust",
            "printf to go",
        ],
    ) || (lower.contains("printf")
        && (lower.contains("explain")
            || lower.contains("simulate")
            || lower.contains("validate")
            || lower.contains("convert")))
}

/// Returns true when the user wants to render an ASCII/Unicode chart from data — steer toward `ascii_chart_tools`.
pub fn needs_ascii_chart_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ascii chart",
            "ascii bar chart",
            "ascii line chart",
            "ascii scatter",
            "ascii plot",
            "text chart",
            "text graph",
            "terminal chart",
            "terminal graph",
            "terminal plot",
            "sparkline",
            "spark line",
            "render chart",
            "plot data",
            "plot these numbers",
            "bar chart from",
            "line chart from",
            "scatter plot",
            "visualize data",
            "chart these values",
            "chart this data",
            "graph these numbers",
            "unicode chart",
            "tui chart",
        ],
    ) || (lower.contains("chart")
        && (lower.contains("bar") || lower.contains("line") || lower.contains("scatter")))
        || (lower.contains("plot")
            && (lower.contains("data") || lower.contains("numbers") || lower.contains("values")))
}

/// Returns true when the user wants to format, minify, split, or extract from SQL — steer toward `sql_format_tools`.
pub fn needs_sql_format_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "format sql",
            "format this sql",
            "format the sql",
            "beautify sql",
            "pretty print sql",
            "pretty-print sql",
            "minify sql",
            "sql formatter",
            "sql format",
            "sql beautifier",
            "sql pretty",
            "sql minify",
            "split sql",
            "split into statements",
            "extract tables from sql",
            "extract columns from sql",
            "extract aliases from sql",
            "sql extract",
            "clean up sql",
            "indent sql",
            "normalize sql whitespace",
        ],
    ) || (lower.contains("sql")
        && (lower.contains("format")
            || lower.contains("beautif")
            || lower.contains("indent")
            || lower.contains("minif")))
}

/// Returns true when the user wants to generate or verify a TOTP/HOTP one-time password — steer toward `totp_tools`.
pub fn needs_totp_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "totp",
            "hotp",
            "one-time password",
            "one time password",
            "otp code",
            "authenticator code",
            "google authenticator",
            "2fa code",
            "two-factor code",
            "two factor code",
            "mfa code",
            "generate otp",
            "verify otp",
            "otpauth",
            "otpauth://",
            "rfc 6238",
            "rfc 4226",
            "time-based otp",
            "time based otp",
            "hmac-based otp",
            "authenticator app secret",
            "totp secret",
            "verify 2fa",
        ],
    )
}

pub fn needs_tar_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "tar archive",
            ".tar file",
            "tar file",
            "inspect tar",
            "list tar",
            "tar entries",
            "tar contents",
            "extract from tar",
            "read tar",
            "parse tar",
            "tarball",
            "tar ball",
            "untar",
            "tar listing",
        ],
    ) || (lower.contains(".tar")
        && contains_any(
            &lower,
            &[
                "list", "inspect", "extract", "find", "contents", "info", "what",
            ],
        ))
}

pub fn needs_email_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "parse email",
            "parse eml",
            ".eml file",
            "eml file",
            "inspect eml",
            "eml headers",
            "email headers",
            "email header",
            "email structure",
            "mime structure",
            "mime parts",
            "mime boundary",
            "delivery trace",
            "email trace",
            "received headers",
            "email delivery",
            "rfc 2822",
            "rfc2822",
            "raw email",
            "decode email",
            "analyze email",
            "email attachments",
            "email body",
            "dkim header",
            "spf result",
            "authentication-results",
        ],
    ) || (lower.contains("email")
        && contains_any(
            &lower,
            &[
                "parse",
                "inspect",
                "trace",
                "headers",
                "structure",
                "decode",
                "analyze",
            ],
        ))
}

pub fn needs_cbor_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "cbor",
            "concise binary object",
            "webauthn attestation",
            "webauthn cbor",
            "fido2 cbor",
            "passkey cbor",
            "decode cbor",
            "parse cbor",
            "inspect cbor",
            "cbor binary",
            "cbor hex",
            "cbor format",
            "cbor data",
            "coap",
        ],
    )
}

pub fn needs_msgpack_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "messagepack",
            "message pack",
            "msgpack",
            "msg pack",
            ".msgpack",
            "decode msgpack",
            "parse msgpack",
            "inspect msgpack",
            "msgpack binary",
            "msgpack hex",
            "msgpack format",
        ],
    )
}

pub fn needs_wasm_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".wasm",
            "wasm file",
            "wasm binary",
            "webassembly",
            "web assembly",
            "wasm sections",
            "wasm imports",
            "wasm exports",
            "wasm module",
            "wasm inspector",
            "inspect wasm",
            "parse wasm",
            "analyze wasm",
        ],
    ) || (lower.contains("wasm")
        && contains_any(
            &lower,
            &["list", "inspect", "info", "imports", "exports", "sections"],
        ))
}

pub fn needs_jsonschema_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "json schema",
            "jsonschema",
            "json-schema",
            "validate json",
            "validate against schema",
            "schema validation",
            "json validation",
            "$ref",
            "$defs",
            "draft-07",
            "draft 7",
            "openapi schema",
            "json schema properties",
            "schema properties",
            "required fields in schema",
            "schema refs",
            "schema info",
            "inspect schema",
            "analyze schema",
        ],
    ) || (lower.contains("schema")
        && contains_any(
            &lower,
            &[
                "validate",
                "inspect",
                "properties",
                "refs",
                "analyze",
                "parse",
                "info",
            ],
        ))
}

pub fn needs_html_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "html file",
            "parse html",
            "analyze html",
            "html links",
            "html images",
            "html forms",
            "html tables",
            "html scripts",
            "validate html",
            "html stats",
            "strip html",
            "html to text",
            "extract links from html",
            "extract images from html",
            "html document",
            "html structure",
            "html accessibility",
            "html seo",
            ".html",
            ".htm",
        ],
    ) || (lower.contains("html")
        && contains_any(
            &lower,
            &[
                "parse", "links", "images", "forms", "tables", "validate", "stats", "text",
                "strip", "extract", "analyze", "inspect",
            ],
        ))
}

pub fn needs_vcf_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "vcard",
            "vcf file",
            ".vcf",
            "contact file",
            "parse vcard",
            "parse vcf",
            "vcard contacts",
            "address book",
            "contact import",
            "contact export",
            "vcard to json",
            "vcf to csv",
            "vcard 3.0",
            "vcard 4.0",
            "vcard 2.1",
        ],
    )
}

pub fn needs_network_header_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "network header",
            "parse header bytes",
            "decode ipv4 header",
            "decode ipv6 header",
            "decode tcp header",
            "decode udp header",
            "decode icmp",
            "decode ethernet",
            "raw packet",
            "packet header",
            "ethernet frame",
            "ethernet header",
            "ipv4 checksum",
            "tcp flags",
            "tcp segment",
            "udp datagram",
            "icmp packet",
            "protocol header",
            "hex packet",
            "packet bytes",
            "wireshark hex",
            "hex dump packet",
            "parse raw bytes",
            "ip header",
        ],
    )
}

pub fn needs_tlv_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "tlv",
            "type-length-value",
            "type length value",
            "ber encoding",
            "der encoding",
            "asn.1",
            "asn1",
            "dhcp options",
            "dhcp option bytes",
            "802.11 ie",
            "wifi ie",
            "information element",
            "tlv parse",
            "tlv decode",
            "parse tlv",
            "decode tlv",
            "build tlv",
            "tlv bytes",
            "tlv structure",
            "ber decode",
            "der decode",
            "parse ber",
            "parse der",
            "asn.1 decode",
            "asn1 decode",
        ],
    )
}

pub fn needs_bin_pack_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pack binary",
            "unpack binary",
            "struct pack",
            "struct unpack",
            "binary struct",
            "pack bytes",
            "unpack bytes",
            "pack format",
            "binary format string",
            "pack values into bytes",
            "encode binary data",
            "decode binary data",
            "binary packing",
            "binary unpacking",
            "little-endian pack",
            "big-endian pack",
            "byte packing",
            "byte unpacking",
            "pack fields",
            "binary serialization",
        ],
    )
}

pub fn needs_elf_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "elf binary",
            "elf file",
            "elf header",
            ".elf",
            "executable and linkable",
            "elf sections",
            "elf segments",
            "elf symbols",
            "elf dynamic",
            "readelf",
            "shared library info",
            ".so file",
            "linux binary",
            "linux executable",
            "program headers",
            "section headers",
            "elf entry point",
            "elf machine type",
            "elf class",
            "inspect binary",
            "analyze binary",
            "binary header",
            "object file",
            ".o file",
            "kernel module",
            ".ko file",
            "dynamic linking",
            "needed libraries",
            "elf symbol table",
        ],
    )
}

pub fn needs_asn1_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "asn.1",
            "asn1",
            "der encoded",
            "ber encoded",
            "der format",
            "ber format",
            "parse der",
            "decode der",
            "der certificate",
            "asn.1 structure",
            "asn.1 tag",
            "tlv der",
            "oid lookup",
            "lookup oid",
            "x.509 der",
            "pkcs der",
            "der binary",
            "asn decode",
        ],
    )
}

pub fn needs_jsonl_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "jsonl",
            "ndjson",
            "json lines",
            "json line",
            "newline delimited json",
            "newline-delimited json",
            "parse jsonl",
            "filter jsonl",
            "aggregate jsonl",
            "jsonl file",
            ".jsonl",
            ".ndjson",
            "jsonl records",
            "json stream",
            "stream of json",
            "log jsonl",
            "jsonl stats",
            "jsonl to csv",
        ],
    )
}

pub fn needs_leb128_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "leb128",
            "uleb128",
            "sleb128",
            "leb encoding",
            "variable length integer",
            "variable-length integer",
            "variable length encoding",
            "varint",
            "wasm encoding",
            "dwarf encoding",
            "protobuf encoding",
            "encode leb",
            "decode leb",
            "leb decode",
            "leb encode",
            "little endian base 128",
            "little-endian base-128",
        ],
    )
}

pub fn needs_unicode_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "unicode analysis",
            "unicode script",
            "unicode block",
            "unicode bidi",
            "bidi control",
            "trojan source",
            "homoglyph",
            "confusable character",
            "unicode confusable",
            "unicode normalization",
            "nfc normalization",
            "nfd normalization",
            "unicode encoding",
            "utf-8 bytes",
            "utf-16 bytes",
            "utf-32 bytes",
            "unicode codepoint analysis",
            "rtl character",
            "right-to-left override",
            "unicode security",
            "analyze unicode",
            "unicode text analysis",
            "unicode inspect",
            "character scripts",
            "codepoint distribution",
        ],
    )
}

pub fn needs_todo_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "todo",
            "fixme",
            "hack comment",
            "code annotation",
            "find todos",
            "scan for todos",
            "list todos",
            "find fixme",
            "scan fixme",
            "annotated comment",
            "code comment scan",
            "deprecated comment",
            "optimize comment",
            "workaround comment",
            "kludge",
            "technical debt comment",
        ],
    )
}

pub fn needs_grep_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "grep for",
            "grep files",
            "search files for",
            "search code for",
            "search codebase for",
            "find in files",
            "find text in",
            "search for pattern",
            "regex search",
            "find pattern in",
            "search source for",
            "look for pattern",
            "find occurrences of",
            "find all occurrences",
            "rg ",
            "ripgrep",
        ],
    )
}

pub fn needs_file_tree_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "file tree",
            "directory tree",
            "folder tree",
            "tree view",
            "show directory structure",
            "show folder structure",
            "directory structure",
            "dir structure",
            "list directory tree",
            "generate tree",
            "ascii tree",
            "project structure",
            "tree command",
            "visualize directory",
            "visualize folder",
        ],
    )
}

pub fn needs_find_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "find files",
            "find all files",
            "find file named",
            "find files named",
            "find files matching",
            "find files with extension",
            "files larger than",
            "files bigger than",
            "files smaller than",
            "recently modified files",
            "recently changed files",
            "find files modified",
            "find command",
            "list files matching",
            "search for files",
        ],
    )
}

pub fn needs_text_extract_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "extract emails",
            "extract email addresses",
            "extract urls",
            "extract links",
            "extract ip addresses",
            "extract ips",
            "extract phone numbers",
            "extract phones",
            "extract dates",
            "extract uuids",
            "extract hashes",
            "extract entities",
            "find emails in",
            "find urls in",
            "find ip addresses in",
            "find phone numbers in",
            "pull emails from",
            "pull urls from",
            "regex extraction",
            "custom pattern extract",
            "extract all entities",
            "scan for emails",
            "scan for urls",
        ],
    )
}

pub fn needs_interval_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "date interval",
            "interval overlap",
            "do intervals overlap",
            "dates overlap",
            "date range overlap",
            "overlapping dates",
            "date range contains",
            "is date within",
            "date within range",
            "merge intervals",
            "merge date ranges",
            "union of intervals",
            "intersect intervals",
            "intersection of dates",
            "date schedule",
            "generate schedule",
            "recurring dates",
            "date sequence",
            "duration between dates",
            "time between dates",
            "days between dates",
            "how many days between",
        ],
    )
}

pub fn needs_inflect_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pluralize this",
            "plural form of",
            "plural of",
            "make plural",
            "singularize",
            "singular form of",
            "singular of",
            "pluralize_with",
            "verb conjugat",
            "third person singular",
            "present participle",
            "past tense of",
            "verb past tense",
            "possessive form",
            "noun possessive",
            "inflect this word",
            "word inflection",
            "english inflection",
        ],
    )
}

pub fn needs_text_align_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "align text",
            "text alignment",
            "right align",
            "center align",
            "left align",
            "justify text",
            "justify the text",
            "align columns",
            "column layout",
            "format columns",
            "side by side columns",
            "add indentation",
            "remove indentation",
            "indent the lines",
            "normalize whitespace",
            "normalize spacing",
            "center this block",
            "alignment ruler",
            "character ruler",
        ],
    )
}

pub fn needs_number_sequence_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "number sequence",
            "numeric sequence",
            "sequence pattern",
            "detect sequence",
            "identify sequence",
            "continue the sequence",
            "extend the sequence",
            "next terms",
            "next numbers in",
            "what comes next in",
            "difference table",
            "arithmetic sequence",
            "geometric sequence",
            "fibonacci sequence",
            "triangular numbers",
            "sequence stats",
            "analyze this sequence",
            "what is the pattern in",
        ],
    )
}

pub fn needs_number_words_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "number to words",
            "number in words",
            "spell out the number",
            "spell the number",
            "write the number in english",
            "number as words",
            "ordinal number",
            "ordinal form",
            "first second third",
            "words to number",
            "parse number words",
            "number in english",
            "english words for",
            "currency words",
            "amount in words",
            "spell digits",
            "say the digits",
            "roman numeral",
            "convert to roman",
            "roman to integer",
            "from roman numeral",
        ],
    )
}

/// Returns true when the user wants music theory calculations — steer toward `music_tools`.
pub fn needs_music_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "music note",
            "note frequency",
            "note to frequency",
            "frequency to note",
            "frequency of a",
            "frequency of c",
            "frequency of d",
            "frequency of e",
            "frequency of f",
            "frequency of g",
            "frequency of b",
            "musical note",
            "note name",
            "a4 440",
            "440 hz",
            "midi note",
            "midi number",
            "note number",
            "music chord",
            "chord notes",
            "chord quality",
            "major chord",
            "minor chord",
            "diminished chord",
            "augmented chord",
            "dominant seventh",
            "music scale",
            "major scale",
            "minor scale",
            "pentatonic scale",
            "blues scale",
            "dorian scale",
            "phrygian",
            "lydian scale",
            "mixolydian",
            "chromatic scale",
            "whole tone scale",
            "music interval",
            "perfect fifth",
            "major third",
            "minor third",
            "semitone",
            "bpm to ms",
            "tempo calculation",
            "note duration at",
            "beats per minute",
            "quarter note ms",
            "detect chord",
            "what chord is",
            "identify chord",
        ],
    )
}

/// Returns true when the user wants propositional logic operations — steer toward `logic_tools`.
pub fn needs_logic_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "truth table",
            "propositional logic",
            "boolean logic",
            "boolean expression",
            "logic expression",
            "logic formula",
            "satisfiable",
            "satisfiability",
            "tautology",
            "contradiction",
            "logical tautology",
            "cnf form",
            "dnf form",
            "conjunctive normal form",
            "disjunctive normal form",
            "logical and",
            "logical or",
            "logical not",
            "logical xor",
            "implies expression",
            "biconditional",
            "iff expression",
            "evaluate logic",
            "logic gate",
            "boolean satisf",
            "sat problem",
            "p implies q",
            "p -> q",
            "a and b or",
            "not a and",
            "simplify boolean",
            "logical equivalence",
        ],
    )
}

/// Returns true when the user wants periodic table lookups or molar mass — steer toward `periodic_tools`.
pub fn needs_periodic_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "periodic table",
            "element symbol",
            "atomic number",
            "atomic mass",
            "atomic weight",
            "molar mass",
            "molecular weight",
            "periodic element",
            "chemical element",
            "electronegativity",
            "electron config",
            "melting point of",
            "boiling point of",
            "element density",
            "noble gas",
            "alkali metal",
            "alkaline earth",
            "transition metal",
            "halogen element",
            "lanthanide",
            "actinide",
            "what element is",
            "element group",
            "element period",
            "what is h2o",
            "molar mass of",
            "mass of h2",
            "mass of co2",
            "mass of nacl",
            "formula mass",
            "molecular formula mass",
        ],
    )
}

/// Returns true when the user wants 2D/3D vector math operations — steer toward `vector_tools`.
pub fn needs_vector_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "dot product",
            "cross product",
            "vector magnitude",
            "vector length",
            "normalize vector",
            "unit vector",
            "vector addition",
            "add vectors",
            "subtract vectors",
            "scale vector",
            "angle between vectors",
            "vector projection",
            "project vector",
            "reflect vector",
            "vector math",
            "2d vector",
            "3d vector",
            "orthogonal vectors",
            "perpendicular vectors",
            "parallel vectors",
            "vector norm",
            "euclidean norm",
            "vector operations",
            "scalar multiplication",
        ],
    )
}

pub fn needs_physics_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "physics constant",
            "physical constant",
            "speed of light",
            "planck constant",
            "boltzmann constant",
            "avogadro",
            "gravitational constant",
            "elementary charge",
            "vacuum permittivity",
            "coulombs constant",
            "faraday constant",
            "stefan-boltzmann",
            "bohr radius",
            "fine structure constant",
            "rydberg constant",
            "physics formula",
            "kinetic energy formula",
            "ohms law",
            "ideal gas formula",
            "coulombs law",
            "snells law",
            "de broglie",
            "carnot efficiency",
            "thin lens",
            "wave speed formula",
            "photon energy",
            "centripetal force",
            "gravitational force formula",
            "heat capacity formula",
            "momentum formula",
            "mass energy",
            "e=mc",
            "f=ma",
            "pv=nrt physics",
            "kinematics formula",
            "physics domains",
            "list physics",
        ],
    )
}

pub fn needs_chemistry_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "balance chemical",
            "balance equation",
            "balance reaction",
            "chemical equation",
            "stoichiometry",
            "mole ratio",
            "molar mass of",
            "molarity",
            "dilution formula",
            "c1v1",
            "c1v1=c2v2",
            "henderson-hasselbalch",
            "henderson hasselbalch",
            "buffer ph",
            "ph of buffer",
            "ph calculation",
            "poh calculation",
            "acid dissociation",
            "ka to ph",
            "kb to pkb",
            "ideal gas law chemistry",
            "pv=nrt chemistry",
            "gas law",
            "gas pressure",
            "gas volume",
            "gas moles",
            "chemical formula mass",
            "atomic mass",
            "formula weight",
            "limiting reagent",
            "percent yield",
            "solution concentration",
            "moles of solute",
        ],
    )
}

pub fn needs_notebook_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "jupyter notebook",
            "jupyter file",
            ".ipynb",
            "ipynb file",
            "parse notebook",
            "notebook cells",
            "notebook outputs",
            "notebook source",
            "notebook stats",
            "extract notebook",
            "analyze notebook",
            "inspect notebook",
            "list cells",
            "code cells",
            "notebook metadata",
            "jupyter kernel",
        ],
    )
}

pub fn needs_conda_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "conda environment",
            "environment.yml",
            "conda env",
            "parse conda",
            "conda dependencies",
            "conda packages",
            "conda channels",
            "compare conda",
            "validate conda",
            "export conda",
            "conda to pip",
            "conda requirements",
            "conda yml",
            "conda yaml",
            "anaconda environment",
            "miniconda env",
        ],
    )
}

pub fn needs_cite_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "cite this",
            "citation for",
            "format citation",
            "apa citation",
            "mla citation",
            "chicago citation",
            "ieee citation",
            "harvard citation",
            "generate bibtex",
            "bibtex entry",
            "format reference",
            "academic citation",
            "reference list",
            "bibliography entry",
            "in-text citation",
            "cite source",
            "doi citation",
            "isbn citation",
            "validate citation",
            "citation style",
            "format a reference",
            "journal citation",
            "book citation",
            "website citation",
            "conference citation",
            "thesis citation",
            "parse doi",
            "validate isbn",
            "doi to citation",
        ],
    )
}

pub fn needs_latex_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "latex",
            "latex table",
            "latex equation",
            "latex template",
            "escape latex",
            "latex symbol",
            "latex math",
            "latex document",
            "latex code",
            "latex formula",
            "latex syntax",
            "latex beamer",
            "latex article",
            "latex report",
            "convert to latex",
            "markdown to latex",
            "strip latex",
            "remove latex",
            "latex align",
            "latex equation environment",
            "\\begin{",
            "\\documentclass",
            "\\usepackage",
            "latex special characters",
            "latex escape",
        ],
    )
}

pub fn needs_astro_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "planet position",
            "planetary position",
            "heliocentric",
            "geocentric",
            "planet longitude",
            "rise and set",
            "rise/set time",
            "star rise",
            "angular separation",
            "sky separation",
            "celestial separation",
            "apparent magnitude",
            "stellar magnitude",
            "astronomical magnitude",
            "astronomical unit",
            "light year",
            "parsec distance",
            "constellation",
            "iau constellation",
            "moon phase",
            "lunar phase",
            "julian date",
            "julian day",
            "jd to date",
            "date to jd",
            "j2000",
            "ephemeris",
            "astronomy",
            "celestial",
            "right ascension",
            "declination",
            "hour angle",
        ],
    )
}

pub fn needs_signal_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "discrete fourier",
            "dft of",
            "idft",
            "inverse dft",
            "fft of",
            "fir filter",
            "fir design",
            "lowpass filter",
            "highpass filter",
            "bandpass filter",
            "bandstop filter",
            "window function",
            "hamming window",
            "hanning window",
            "blackman window",
            "kaiser window",
            "bartlett window",
            "convolve signal",
            "signal convolution",
            "resample signal",
            "upsample",
            "downsample",
            "autocorrelation",
            "signal statistics",
            "signal power",
            "rms of signal",
            "zero crossing",
            "sinc filter",
            "signal processing",
            "digital filter",
            "frequency response",
        ],
    )
}

pub fn needs_thermo_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ideal gas",
            "pv=nrt",
            "pv = nrt",
            "gas law",
            "isothermal",
            "isobaric",
            "isochoric",
            "adiabatic process",
            "thermodynamic work",
            "entropy change",
            "heat conduction",
            "fourier's law",
            "thermal conductivity",
            "heat convection",
            "heat radiation",
            "stefan-boltzmann",
            "carnot cycle",
            "carnot efficiency",
            "otto cycle",
            "diesel cycle",
            "brayton cycle",
            "thermodynamic cycle",
            "reynolds number",
            "bernoulli equation",
            "bernoulli's",
            "poiseuille",
            "fluid flow",
            "flow velocity",
            "laminar flow",
            "turbulent flow",
            "psychrometrics",
            "relative humidity",
            "dew point",
            "wet bulb",
            "specific heat",
            "heat capacity ratio",
            "cp/cv",
            "thermodynamics",
            "thermo calculation",
        ],
    )
}

pub fn needs_optics_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "snell's law",
            "snells law",
            "refraction",
            "refractive index",
            "critical angle",
            "total internal reflection",
            "thin lens",
            "focal length",
            "lensmaker",
            "lens equation",
            "lens maker",
            "mirror equation",
            "concave mirror",
            "convex mirror",
            "image distance",
            "object distance",
            "magnification optics",
            "single slit diffraction",
            "single-slit diffraction",
            "diffraction grating",
            "diffraction pattern",
            "double slit",
            "young's experiment",
            "interference pattern",
            "fringe spacing",
            "thin film interference",
            "malus's law",
            "brewster angle",
            "polarized light",
            "optical fiber",
            "numerical aperture",
            "fiber optic",
            "fibre optic",
            "acceptance angle",
            "blackbody radiation",
            "planck's law",
            "wien's law",
            "wien displacement",
            "spectral radiance",
            "color temperature",
            "optics calculation",
            "photon energy optics",
        ],
    )
}

pub fn needs_mechanics_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "kinematics",
            "suvat",
            "projectile motion",
            "projectile range",
            "max height projectile",
            "time of flight",
            "centripetal force",
            "centripetal acceleration",
            "circular motion",
            "moment of inertia",
            "torque calculation",
            "angular acceleration",
            "rotational kinetic energy",
            "angular momentum",
            "simple harmonic motion",
            "shm period",
            "spring period",
            "pendulum period",
            "spring constant",
            "oscillation frequency",
            "elastic collision",
            "inelastic collision",
            "conservation of momentum",
            "conservation of energy",
            "kinetic energy formula",
            "gravitational potential energy",
            "work done by force",
            "power mechanics",
            "newton's second law",
            "newtons second law",
            "friction force",
            "normal force incline",
            "inclined plane",
            "orbital speed",
            "orbital period",
            "classical mechanics",
        ],
    )
}

pub fn needs_circuit_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ohm's law",
            "ohms law",
            "v=ir",
            "resistance series",
            "resistance parallel",
            "resistors in series",
            "resistors in parallel",
            "series resistors",
            "parallel resistors",
            "electrical power",
            "i squared r",
            "p=iv",
            "p=i2r",
            "rc circuit",
            "rl circuit",
            "rlc circuit",
            "resonant frequency circuit",
            "q-factor circuit",
            "impedance",
            "voltage divider",
            "current divider",
            "capacitor energy",
            "capacitor series",
            "capacitor parallel",
            "inductor energy",
            "inductor series",
            "inductor parallel",
            "rc time constant",
            "rl time constant",
            "power factor",
            "ac impedance",
            "reactive power",
            "apparent power",
            "xl impedance",
            "xc impedance",
            "circuit analysis",
            "capacitance calculation",
            "inductance calculation",
        ],
    )
}

pub fn needs_quantum_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "quantum",
            "particle in a box",
            "infinite square well",
            "hydrogen energy level",
            "rydberg",
            "heisenberg uncertainty",
            "uncertainty principle",
            "de broglie",
            "de broglie wavelength",
            "wave-particle",
            "photoelectric effect",
            "work function",
            "compton scattering",
            "compton wavelength",
            "quantum tunneling",
            "quantum harmonic oscillator",
            "zero-point energy",
            "energy quantization",
            "photon energy hf",
            "planck's equation",
        ],
    )
}

pub fn needs_em_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "coulomb's law",
            "coulombs law",
            "electric force",
            "electric field point charge",
            "electric potential energy",
            "magnetic field wire",
            "magnetic field solenoid",
            "magnetic field loop",
            "parallel plate capacitance",
            "cylindrical capacitance",
            "spherical capacitance",
            "solenoid inductance",
            "toroid inductance",
            "coaxial inductance",
            "electromagnetic wave",
            "em wave",
            "lorentz force",
            "lorentz law",
            "poynting vector",
            "radiation pressure",
            "em energy density",
            "electromagnetism",
            "maxwell's equations",
            "gauss's law",
            "faraday's law",
            "ampere's law",
        ],
    )
}

pub fn needs_relativity_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "special relativity",
            "lorentz factor",
            "lorentz boost",
            "time dilation",
            "length contraction",
            "relativistic energy",
            "relativistic momentum",
            "lorentz transformation",
            "relativistic doppler",
            "spacetime interval",
            "proper time",
            "gamma factor relativity",
            "velocity addition relativistic",
            "relativistic kinematics",
            "twin paradox",
            "e=mc2",
            "e=mc²",
            "rest energy",
            "relativistic mass",
            "minkowski",
            "four-momentum",
            "4-momentum",
        ],
    )
}

pub fn needs_nuclear_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "radioactive decay",
            "half-life",
            "halflife",
            "nuclear binding energy",
            "binding energy nucleus",
            "bethe-weizsacker",
            "liquid drop model",
            "q-value nuclear",
            "nuclear reaction q",
            "radiation dose",
            "sievert dose",
            "gray dose",
            "rem dose",
            "becquerel",
            "curie activity",
            "carbon dating",
            "radiocarbon",
            "c-14 dating",
            "c14 dating",
            "nuclear fission energy",
            "nuclear fusion energy",
            "alpha decay",
            "beta decay",
            "decay constant",
            "mean lifetime radioactive",
            "radioactivity",
            "semi-empirical mass formula",
            "semf",
        ],
    )
}

pub fn needs_cors_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "cors header",
            "cors policy",
            "cors config",
            "cors validation",
            "access-control-allow-origin",
            "access-control-allow-methods",
            "access-control-allow-headers",
            "access-control-expose-headers",
            "access-control-allow-credentials",
            "access-control-max-age",
            "allow credentials cors",
            "generate cors",
            "preflight request",
            "preflight cors",
            "cors preflight",
            "cors response headers",
            "parse cors",
            "explain cors",
            "validate cors",
            "same-origin policy",
            "cross-origin resource sharing",
        ],
    )
}

pub fn needs_web_manifest_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "web manifest",
            "manifest.json",
            ".webmanifest",
            "pwa manifest",
            "web app manifest",
            "parse manifest",
            "validate manifest",
            "manifest icons",
            "manifest screenshots",
            "manifest display",
            "manifest orientation",
            "manifest start_url",
            "manifest theme_color",
            "installable pwa",
            "pwa installability",
            "add to home screen",
            "maskable icon",
            "manifest shortcuts",
            "manifest share_target",
        ],
    )
}

pub fn needs_json_patch_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "json patch",
            "rfc 6902",
            "apply patch",
            "json diff",
            "json pointer",
            "json pointer path",
            "merge patch",
            "rfc 7396",
            "json merge",
            "patch document",
            "json operations",
            "add remove replace",
            "json operation",
        ],
    )
}

pub fn needs_markdown_gen_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "generate markdown",
            "markdown table",
            "create markdown table",
            "markdown badge",
            "shields.io badge",
            "generate badge",
            "markdown toc",
            "table of contents markdown",
            "markdown admonition",
            "github admonition",
            "note warning tip",
            "[!note]",
            "[!warning]",
            "[!tip]",
            "markdown link",
            "image link markdown",
            "generate markdown doc",
            "markdown document",
            "markdown section",
            "build markdown",
        ],
    )
}

pub fn needs_sort_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "sort this list",
            "sort these numbers",
            "sort algorithm",
            "sorting algorithm",
            "bubble sort",
            "merge sort",
            "quick sort",
            "heap sort",
            "insertion sort",
            "selection sort",
            "shell sort",
            "counting sort",
            "radix sort",
            "compare sort",
            "sort and compare",
            "sorting steps",
            "binary search trace",
            "sort step by step",
            "sort visualization",
            "best sorting algorithm",
            "sort complexity",
        ],
    )
}

pub fn needs_compression_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "run-length encoding",
            "rle encode",
            "rle decode",
            "lz77",
            "lz compression",
            "compress this text",
            "text compression",
            "huffman coding",
            "huffman encoding",
            "huffman tree",
            "shannon entropy",
            "compressibility",
            "compression ratio",
            "encode with rle",
            "compress with lz",
            "entropy of text",
            "lossless compression",
        ],
    )
}

pub fn needs_trie_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "trie",
            "prefix tree",
            "autocomplete words",
            "word autocomplete",
            "prefix search",
            "words with prefix",
            "build trie",
            "trie search",
            "trie autocomplete",
            "trie structure",
            "prefix lookup",
            "typo suggestions",
            "edit distance suggest",
            "insert words trie",
            "word prefix",
        ],
    )
}

pub fn needs_stack_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "stack data structure",
            "lifo stack",
            "push and pop",
            "stack operations",
            "simulate stack",
            "queue data structure",
            "fifo queue",
            "enqueue dequeue",
            "queue operations",
            "deque data structure",
            "double-ended queue",
            "evaluate expression",
            "rpn expression",
            "reverse polish",
            "infix expression",
            "shunting yard",
            "bracket balance",
            "parenthesis balance",
            "balanced brackets",
            "check parentheses",
            "expression evaluation",
            "postfix expression",
        ],
    )
}

pub fn needs_acoustics_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "sound wave",
            "sound frequency",
            "acoustic",
            "acoustics",
            "decibel",
            "decibels",
            "sound level",
            "spl ",
            "dbspl",
            "sound pressure",
            "doppler sound",
            "doppler effect sound",
            "resonance frequency",
            "standing wave",
            "acoustic resonance",
            "fundamental frequency pipe",
            "acoustic impedance",
            "sound transmission",
            "rt60",
            "reverberation time",
            "room acoustics",
            "sabine formula",
            "hearing range",
            "audible frequency",
            "threshold of hearing",
            "beat frequency",
            "beat note",
            "overtone series",
            "harmonic series sound",
            "speed of sound",
        ],
    )
}

pub fn needs_materials_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "material properties",
            "young's modulus",
            "youngs modulus",
            "elastic modulus",
            "poisson's ratio",
            "poisson ratio",
            "yield strength",
            "tensile strength",
            "stress strain",
            "thermal expansion",
            "coefficient of expansion",
            "linear expansion cte",
            "beam bending",
            "bending stress",
            "bending moment",
            "moment of inertia beam",
            "section modulus",
            "mohs hardness",
            "material hardness",
            "vickers hardness",
            "brinell hardness",
            "buoyancy force",
            "buoyant force",
            "archimedes principle",
            "hydrostatic pressure",
            "factor of safety",
            "safety factor",
            "crystal structure",
            "fcc crystal",
            "bcc crystal",
            "hcp crystal",
            "unit cell material",
            "lattice parameter",
            "atomic packing",
        ],
    )
}

pub fn needs_pe_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pe binary",
            "pe file",
            "pe header",
            "pe format",
            ".exe binary",
            ".dll binary",
            ".dll imports",
            ".dll exports",
            ".sys file",
            ".ocx file",
            "windows executable",
            "windows binary",
            "windows pe",
            "pe sections",
            "pe imports",
            "pe exports",
            "pe32+",
            "dumpbin",
            "readpe",
            "inspect exe",
            "inspect dll",
            "analyze exe",
            "analyze dll",
            "aslr enabled",
            "dep enabled",
            "guard cf",
            "nx_compat",
            "dynamic_base",
            "dll characteristics",
            "image base",
            "entry point rva",
            "coff header",
            "optional header pe",
        ],
    )
}

pub fn needs_macho_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "mach-o",
            "macho",
            "mach o binary",
            ".dylib",
            ".dylib binary",
            "dylib imports",
            "dylib info",
            "macos binary",
            "macos executable",
            "apple binary",
            "fat binary",
            "universal binary",
            "otool",
            "inspect dylib",
            "analyze dylib",
            "inspect macho",
            "analyze macho",
            "mach-o segments",
            "mach-o sections",
            "mach-o imports",
            "mach-o fat",
            "lc_load_dylib",
            "lc_segment",
            "feedface",
            "feedfacf",
            "cafebabe fat",
            "arm64 binary",
            "x86-64 macos",
        ],
    )
}

pub fn needs_pcap_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pcap",
            "pcapng",
            "packet capture",
            "packet capture file",
            "wireshark",
            "tcpdump",
            "network capture",
            "analyze pcap",
            "parse pcap",
            "inspect pcap",
            ".pcap file",
            ".pcapng file",
            "pcap packets",
            "pcap dns",
            "pcap http",
            "pcap protocol",
            "network traffic analysis",
            "capture file",
        ],
    )
}

pub fn needs_class_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".class file",
            "java class file",
            "java bytecode",
            "jvm bytecode",
            "class bytecode",
            "inspect .class",
            "analyze .class",
            "parse .class",
            "cafebabe",
            "cafe babe",
            "java constant pool",
            "jvm class",
            "class methods",
            "class fields",
            "java class info",
            "java class methods",
            "java class fields",
            "class file version",
            "java major version",
            "java access flags",
            "javap",
            "decompile class",
            "class imports",
            "class references",
            "java compiled",
            "java class expose",
        ],
    )
}

pub fn needs_dex_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".dex file",
            "dex file",
            "android dex",
            "dalvik dex",
            "dalvik executable",
            "inspect dex",
            "analyze dex",
            "parse dex",
            "android bytecode",
            "android classes",
            "android methods",
            "android strings",
            "android apk dex",
            "classes.dex",
            "dex version",
            "dex class",
            "dex method",
            "dex strings",
            "dex types",
            "android reverse",
            "dexdump",
            "baksmali",
        ],
    )
}

pub fn needs_tls_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "tls record",
            "tls handshake",
            "client hello",
            "clienthello",
            "server hello",
            "serverhello",
            "tls cipher",
            "cipher suite",
            "tls extension",
            "tls parse",
            "decode tls",
            "inspect tls",
            "tls bytes",
            "tls hex",
            "tls 1.2",
            "tls 1.3",
            "heartbleed extension",
            "sni extension",
            "alpn extension",
            "supported_versions extension",
            "key_share extension",
            "tls alert",
            "ssl record",
            "ssl handshake",
            "ssl client hello",
            "tls security",
        ],
    )
}

pub fn needs_protobuf_wire_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "protobuf wire",
            "proto wire",
            "protobuf bytes",
            "proto bytes",
            "decode protobuf",
            "decode proto",
            "protobuf hex",
            "proto hex",
            "grpc payload",
            "grpc bytes",
            "grpc decode",
            "wire format protobuf",
            "wire type",
            "varint protobuf",
            "protobuf field",
            "proto field number",
            "length-delimited",
            "protobuf binary",
            "proto binary",
            "raw protobuf",
            "raw proto",
            "protobuf without schema",
            "proto without schema",
        ],
    )
}

pub fn needs_ssh_key_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ssh public key",
            "ssh key fingerprint",
            "authorized_keys",
            "authorized keys",
            ".pub file",
            "ssh-rsa",
            "ssh-ed25519",
            "ecdsa-sha2-nistp",
            "ssh key type",
            "ssh key bits",
            "parse ssh key",
            "inspect ssh key",
            "validate ssh key",
            "ssh key info",
            "ed25519 key",
            "ssh fingerprint",
            "key fingerprint sha",
        ],
    )
}

pub fn needs_wireguard_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "wireguard",
            "wg-quick",
            "wg0.conf",
            "wireguard config",
            "wireguard conf",
            "allowedips",
            "persistentkeepalive",
            "presharedkey",
            "wireguard peer",
            "wireguard key",
            "wireguard tunnel",
            "wireguard vpn",
            "wg peer",
            "wg tunnel",
        ],
    )
}

pub fn needs_prometheus_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "prometheus",
            "openmetrics",
            "metrics exposition",
            "parse metrics",
            "prometheus metrics",
            "metric family",
            "counter metric",
            "gauge metric",
            "histogram metric",
            "summary metric",
            "metrics scrape",
            "prometheus format",
            "# help",
            "# type",
            "metric families",
            "scrape output",
        ],
    )
}

pub fn needs_http_cache_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "cache-control",
            "cache control header",
            "http cache",
            "etag",
            "if-none-match",
            "vary header",
            "max-age",
            "s-maxage",
            "no-cache directive",
            "no-store directive",
            "must-revalidate",
            "stale-while-revalidate",
            "immutable directive",
            "http freshness",
            "cache freshness",
            "conditional request",
            "304 not modified",
            "http caching",
        ],
    )
}

pub fn needs_webhook_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "webhook signature",
            "webhook secret",
            "webhook verify",
            "webhook hmac",
            "github webhook",
            "stripe webhook",
            "slack webhook signature",
            "shopify webhook",
            "x-hub-signature",
            "stripe-signature",
            "x-slack-signature",
            "x-shopify-hmac",
            "hmac webhook",
            "verify webhook",
            "sign webhook",
            "webhook signing",
        ],
    )
}

pub fn needs_jwk_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "jwk",
            "jwks",
            "json web key",
            "rfc 7517",
            "rfc 7638",
            "jwk thumbprint",
            "key thumbprint",
            "jwks endpoint",
            "parse jwk",
            "validate jwk",
            "jwk set",
            "well-known/jwks",
            "jwks.json",
        ],
    )
}

pub fn needs_gitlab_ci_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "gitlab-ci",
            ".gitlab-ci.yml",
            "gitlab ci",
            "gitlab pipeline",
            "gitlab job",
            "gitlab stages",
            "gitlab workflow",
            "ci/cd yaml",
            "parse gitlab",
            "validate gitlab",
            "gitlab runner",
            "gitlab needs",
            "gitlab rules",
            "gitlab artifacts",
            "gitlab before_script",
        ],
    )
}

pub fn needs_junit_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "junit",
            "xunit",
            "test result",
            "test results xml",
            "failing tests",
            "test failures",
            "parse test xml",
            "test report xml",
            "junit xml",
            "test suite xml",
            "test summary xml",
            "testcase xml",
            "testsuite xml",
            "test pass rate",
        ],
    )
}

pub fn needs_ansible_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ansible",
            "ansible playbook",
            "playbook.yml",
            "parse playbook",
            "ansible tasks",
            "ansible plays",
            "ansible vars",
            "ansible handlers",
            "ansible roles",
            "ansible validate",
            "ansible modules",
            "ansible when",
            "ansible tags",
            "ansible become",
            "inspect playbook",
        ],
    )
}

pub fn needs_grpc_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "grpc status",
            "grpc code",
            "grpc error",
            "not_found grpc",
            "unavailable grpc",
            "deadline_exceeded",
            "unauthenticated grpc",
            "permission_denied grpc",
            "grpc metadata",
            "grpc headers",
            "grpc status code",
            "grpc codes",
            "list grpc",
            "explain grpc",
            "grpc retryable",
        ],
    )
}

pub fn needs_haproxy_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "haproxy",
            "haproxy.cfg",
            "haproxy config",
            "parse haproxy",
            "haproxy frontend",
            "haproxy backend",
            "haproxy server",
            "haproxy acl",
            "haproxy balance",
            "haproxy validate",
            "haproxy listen",
            "load balancer config",
            "haproxy global",
            "haproxy defaults",
        ],
    )
}

pub fn needs_helm_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "helm chart",
            "helm values",
            "helm template",
            "chart.yaml",
            "values.yaml helm",
            "parse helm",
            "inspect helm",
            "helm deps",
            "helm dependencies",
            "helm validate",
            "helm package",
            "helm release",
            "helm repo",
            "kubernetes helm",
        ],
    )
}

pub fn needs_cvss_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "cvss",
            "cvss score",
            "cvss vector",
            "cvss v3",
            "cvss:3.",
            "av:n/ac:",
            "base score",
            "vulnerability score",
            "vulnerability severity",
            "cvss calculator",
            "cvss decode",
            "cvss rating",
            "nvd score",
            "cve score",
            "exploitability score",
            "impact score",
        ],
    )
}

pub fn needs_nmap_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "nmap",
            "nmap xml",
            "nmap scan",
            "nmap output",
            "nmap result",
            "parse nmap",
            "nmap report",
            "port scan result",
            "network scan xml",
            "nmap -ox",
        ],
    )
}

pub fn needs_postman_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "postman",
            "postman collection",
            "postman collection.json",
            "postman requests",
            "postman folder",
            "postman api",
            "parse postman",
            "postman export",
            "postman variables",
            "postman environment",
            "api collection",
            "collection.json",
        ],
    )
}

pub fn needs_ldif_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "ldif",
            ".ldif",
            "ldif file",
            "parse ldif",
            "ldap data",
            "ldap export",
            "ldap entries",
            "ldap directory",
            "openldap",
            "active directory ldif",
            "ldap dn",
            "directory information",
            "objectclass ldap",
            "ldap attributes",
        ],
    )
}

pub fn needs_iptables_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "iptables",
            "iptables-save",
            "iptables rules",
            "iptables chain",
            "iptables filter",
            "iptables nat",
            "iptables mangle",
            "ip6tables",
            "parse iptables",
            "firewall rules iptables",
            "netfilter",
            "iptables-restore",
            "iptables policy",
            "linux firewall rules",
        ],
    )
}

pub fn needs_spdx_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "spdx",
            "spdx license",
            "spdx expression",
            "spdx identifier",
            "license expression",
            "license compatibility",
            "parse license expression",
            "validate license expression",
            "license identifier",
            "osi approved",
            "fsf approved",
            "copyleft license",
            "permissive license list",
            "open source license list",
        ],
    )
}

pub fn needs_aws_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "arn:",
            "aws arn",
            "parse arn",
            "decode arn",
            "aws resource name",
            "amazon resource name",
            "s3://",
            "s3 uri",
            "s3 url",
            "aws region",
            "aws service",
            "list aws regions",
            "aws partition",
            "arn partition",
            "s3 bucket url",
            "s3.amazonaws.com",
        ],
    )
}

pub fn needs_curl_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "parse curl",
            "convert curl",
            "curl command",
            "curl to python",
            "curl to go",
            "curl to javascript",
            "curl to js",
            "build curl",
            "generate curl",
            "curl -x",
            "curl --header",
            "curl request",
            "explain curl",
            "curl syntax",
        ],
    )
}

pub fn needs_oauth_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pkce",
            "code verifier",
            "code challenge",
            "oauth",
            "oauth2",
            "oauth 2.0",
            "oauth grant",
            "authorization code flow",
            "client credentials flow",
            "authorization url",
            "oauth url",
            "oauth token",
            "decode oauth",
            "oauth flow",
            "implicit grant",
            "rfc 7636",
            "openid connect",
        ],
    )
}

pub fn needs_saml_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "saml",
            "saml response",
            "saml assertion",
            "parse saml",
            "decode saml",
            "saml token",
            "saml attributes",
            "saml validate",
            "samlresponse",
            "saml2",
            "saml 2.0",
            "identity provider saml",
            "sso saml",
            "saml sso",
            "saml conditions",
            "saml nameidentifier",
        ],
    )
}

pub fn needs_multipart_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "multipart",
            "form-data",
            "multipart/form-data",
            "parse multipart",
            "file upload body",
            "parse form-data",
            "rfc 2046",
            "content-disposition",
            "multipart body",
            "multipart boundary",
            "build multipart",
            "generate multipart",
            "validate multipart",
        ],
    )
}

pub fn needs_openid_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "openid connect",
            "oidc",
            "openid configuration",
            "openid discovery",
            ".well-known/openid",
            "id token",
            "id_token",
            "oidc scope",
            "openid scope",
            "userinfo endpoint",
            "userinfo claims",
            "openid claims",
            "oidc client",
            "openid client",
            "oidc discovery",
            "decode id token",
            "inspect id token",
            "openid token",
        ],
    )
}

pub fn needs_exif_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "exif",
            "exif data",
            "exif metadata",
            "image metadata",
            "jpeg metadata",
            "photo metadata",
            "camera metadata",
            "gps from photo",
            "gps from image",
            "extract gps from",
            "photo location",
            "image location",
            "photo coordinates",
            "tiff metadata",
            "read exif",
            "parse exif",
            "camera model photo",
            "lens info photo",
            "shutter speed photo",
            "aperture photo",
            "iso photo",
        ],
    )
}

pub fn needs_office_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".docx",
            ".xlsx",
            ".pptx",
            "docx file",
            "xlsx file",
            "pptx file",
            "word document",
            "excel workbook",
            "powerpoint",
            "office document",
            "open xml",
            "inspect docx",
            "inspect xlsx",
            "inspect pptx",
            "parse docx",
            "parse xlsx",
            "read docx",
            "read xlsx",
            "office file",
            "extract text from word",
            "extract text from docx",
            "sheet names",
            "slide count",
            "presentation slides",
        ],
    )
}

pub fn needs_font_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "font file",
            ".ttf",
            ".otf",
            ".woff",
            "woff2",
            "truetype",
            "opentype",
            "font metadata",
            "font family",
            "font name",
            "font tables",
            "glyph count",
            "font glyphs",
            "glyphs",
            "inspect font",
            "parse font",
            "font license",
            "font embedding",
            "font copyright",
            "unicode coverage",
            "cmap table",
            "font version",
            "sfnt",
            "font weight",
        ],
    )
}

pub fn needs_svg_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            ".svg",
            "svg file",
            "svg document",
            "svg image",
            "scalable vector",
            "parse svg",
            "inspect svg",
            "svg elements",
            "svg ids",
            "svg viewbox",
            "svg width",
            "svg height",
            "svg namespace",
            "svg validate",
            "svg links",
            "svg styles",
            "svg animation",
            "svg script",
            "svg accessibility",
            "svg xlink",
            "svg structure",
            "vector graphic",
        ],
    )
}

pub fn needs_image_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "image metadata",
            "image file metadata",
            "png metadata",
            "jpeg metadata",
            "gif metadata",
            "webp metadata",
            "bmp metadata",
            "image dimensions",
            "image width",
            "image height",
            "image color",
            "image dpi",
            "image resolution",
            "image alpha",
            "parse image",
            "inspect image",
            "image format",
            "image info",
            "animated gif",
            "apng",
            "gif frames",
            "webp info",
            "webp file",
            "png file",
            "jpeg file",
            "gif file",
            "bmp file",
            "image color mode",
            "color depth image",
            "bit depth image",
            "icc profile image",
            "validate image",
            "image palette",
            "transparency image",
        ],
    )
}

pub fn needs_audio_file_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "audio metadata",
            "audio file metadata",
            "wav metadata",
            "mp3 metadata",
            "flac metadata",
            "ogg metadata",
            "id3 tags",
            "id3 tag",
            "vorbis comment",
            "vorbis tag",
            "audio duration",
            "audio sample rate",
            "audio channels",
            "audio bit depth",
            "audio bitrate",
            "parse mp3",
            "parse wav",
            "parse flac",
            "parse ogg",
            "inspect mp3",
            "inspect wav",
            "inspect flac",
            "inspect ogg",
            "mp3 tags",
            "mp3 info",
            "flac tags",
            "ogg tags",
            "wav info",
            "read id3",
            "audio codec",
            "audio format info",
            "song tags",
            "music tags",
        ],
    )
}

pub fn needs_video_file_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "video metadata",
            "video file metadata",
            "mp4 metadata",
            "mkv metadata",
            "avi metadata",
            "mov metadata",
            "webm metadata",
            "mp4 file",
            "mkv file",
            "avi file",
            "mov file",
            "webm file",
            "parse mp4",
            "parse mkv",
            "parse avi",
            "parse mov",
            "inspect mp4",
            "inspect mkv",
            "inspect avi",
            "video streams",
            "video codec",
            "video duration",
            "video resolution",
            "video container",
            "mp4 info",
            "mkv info",
            "avi info",
            "video frame rate",
            "mp4 streams",
            "mkv streams",
            "matroska",
            "mp4 container",
            "video file info",
        ],
    )
}

pub fn needs_pdf_tools(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    contains_any(
        &lower,
        &[
            "pdf metadata",
            "pdf file metadata",
            "parse pdf",
            "inspect pdf",
            "pdf info",
            "pdf page count",
            "pdf pages",
            "pdf author",
            "pdf title",
            "pdf creator",
            "pdf producer",
            "pdf creation date",
            "pdf structure",
            "pdf version",
            "pdf validate",
            "validate pdf",
            "pdf document",
            "pdf info dict",
            ".pdf file",
            "pdf file",
            "pdf size",
            "pdf linearized",
            "pdf xref",
            "pdf object count",
        ],
    )
}

pub fn needs_epub_tools(user_input: &str) -> bool {
    let s = user_input.to_lowercase();
    let asks_epub = s.contains("epub")
        || s.contains(".epub")
        || s.contains("ebook metadata")
        || s.contains("ebook file")
        || s.contains("kindle book")
        || s.contains("parse epub")
        || s.contains("inspect epub")
        || s.contains("epub metadata")
        || s.contains("epub toc")
        || s.contains("epub table of contents")
        || s.contains("epub spine")
        || s.contains("epub author")
        || s.contains("epub chapters")
        || s.contains("epub validate")
        || s.contains("open ebook")
        || s.contains("oebps")
        || s.contains("opf metadata")
        || s.contains("ncx toc")
        || s.contains("digital book metadata")
        || s.contains("epub version")
        || s.contains("epub publisher");
    asks_epub
}

pub fn needs_sbom_tools(user_input: &str) -> bool {
    let s = user_input.to_lowercase();
    let asks_sbom = s.contains("sbom")
        || s.contains("software bill of materials")
        || s.contains("bill of materials")
        || s.contains("cyclonedx")
        || s.contains("spdx")
        || s.contains("bom.json")
        || s.contains("sbom.json")
        || s.contains(".spdx")
        || s.contains("spdx license")
        || s.contains("spdx document")
        || s.contains("parse sbom")
        || s.contains("inspect sbom")
        || s.contains("sbom components")
        || s.contains("sbom licenses")
        || s.contains("sbom vulnerabilities")
        || s.contains("supply chain")
        || s.contains("component licenses")
        || s.contains("dependency licenses")
        || s.contains("license inventory")
        || s.contains("sbom format")
        || s.contains("software composition")
        || s.contains("purl ecosystem")
        || s.contains("sbom validate")
        || s.contains("bom format");
    asks_sbom
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_query_intent_routes_creator_questions_to_about() {
        let intent = classify_query_intent(WorkflowMode::Auto, "Who created Hematite?");
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::About));

        let intent = classify_query_intent(WorkflowMode::Auto, "/about");
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::About));
    }

    #[test]
    fn classify_query_intent_routes_known_author_question_to_about() {
        let intent = classify_query_intent(WorkflowMode::Auto, "who is ocean bennett");
        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::About));
    }

    #[test]
    fn classify_query_intent_marks_maintainer_workflow_requests() {
        let intent = classify_query_intent(
            WorkflowMode::Auto,
            "Run my cleanup scripts and prune old artifacts.",
        );
        assert!(intent.maintainer_workflow_mode);
        assert_eq!(
            preferred_maintainer_workflow("Rebuild the local portable and update PATH."),
            Some("package_windows")
        );
        assert_eq!(
            preferred_maintainer_workflow("Run the release flow and publish crates."),
            Some("release")
        );
    }

    #[test]
    fn classify_query_intent_marks_workspace_workflow_requests() {
        let intent = classify_query_intent(WorkflowMode::Auto, "Run the tests in this project.");
        assert!(intent.workspace_workflow_mode);
        assert_eq!(
            preferred_workspace_workflow("Run the tests in this project."),
            Some("test")
        );
        assert_eq!(
            preferred_workspace_workflow("Run npm run dev in this repo."),
            Some("script")
        );
    }

    #[test]
    fn test_overclocker_routing() {
        assert_eq!(
            preferred_host_inspection_topic("How's my silicon health looking?"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("Show me GPU clocks"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("nvidia stats"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("Show me GPU voltage telemetry"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("What are my CPU and GPU volts right now?"),
            Some("overclocker")
        );
    }

    #[test]
    fn test_gpu_throttle_routing() {
        assert_eq!(
            preferred_host_inspection_topic("Is my GPU currently throttled and why?"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("Tell me if my GPU is throttled"),
            Some("overclocker")
        );
        assert_eq!(
            preferred_host_inspection_topic("Is the GPU overheating?"),
            Some("overclocker")
        );
    }

    #[test]
    fn test_host_inspection_gateway() {
        assert!(mentions_host_inspection_question("is my gpu throttled?"));
        assert!(mentions_host_inspection_question(
            "check vram and silicon health"
        ));
        assert!(mentions_host_inspection_question("nvidia stats"));

        // Negative tests: General coding/repo questions should NOT trigger the gate
        assert!(!mentions_host_inspection_question("What is a Rust macro?"));
        assert!(!mentions_host_inspection_question(
            "Explain the repository structure."
        ));
        assert!(!mentions_host_inspection_question(
            "is this code efficient?"
        ));
    }

    #[test]
    fn test_web_mutation_routing() {
        // This is the prompt that previously failed by routing to HostInspection
        let input = "I want to change the primary brand color from whatever it is now to a vibrant 'Neon Hematite' (HSL 180, 100%, 50%). Update all CSS variables, update the JS theme toggle logic to support this as the new default highlight, and ensure the HTML icons match. Run verify_build when you are done.";

        // Test in Auto mode (where it should stay in code)
        let intent = classify_query_intent(WorkflowMode::Auto, input);
        assert_eq!(intent.primary_class, QueryIntentClass::Implementation);
        assert_eq!(intent.direct_answer, None);

        // Test in Code mode (where it should stay in code)
        let intent = classify_query_intent(WorkflowMode::Code, input);
        assert_eq!(intent.primary_class, QueryIntentClass::Implementation);
        assert_eq!(intent.direct_answer, None);
    }

    #[test]
    fn test_explicit_diagnostic_during_code() {
        // Even if we are in Code mode, an authoritative hardware noun should trigger the diagnostic
        let input = "Check my GPU stats and tell me if it's throttled.";
        let intent = classify_query_intent(WorkflowMode::Code, input);

        assert_eq!(intent.direct_answer, Some(DirectAnswerKind::HostInspection));
    }

    #[test]
    fn test_coding_shield_logic_collision() {
        // "logic" should not collide with "log" when in code mode
        let input = "Fix the login logic in my typescript code.";
        let intent = classify_query_intent(WorkflowMode::Auto, input);

        assert_eq!(intent.primary_class, QueryIntentClass::Implementation);
        assert_ne!(intent.direct_answer, Some(DirectAnswerKind::HostInspection));
    }

    #[test]
    fn single_file_html_sovereign_prompt_counts_as_scaffold() {
        let input = "google uefn toolbelt then make a folder on my desktop called yourtask and inside it create a single index.html that explains what you found";
        let intent = classify_query_intent(WorkflowMode::Auto, input);

        assert!(intent.sovereign_mode);
        assert!(intent.scaffold_mode);
    }
}
