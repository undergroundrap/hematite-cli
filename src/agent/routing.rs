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
    let ends_confirmation = lower
        .trim_end_matches(|c: char| c == '?' || c == ' ')
        .ends_with("right")
        || lower
            .trim_end_matches(|c: char| c == '?' || c == ' ')
            .ends_with("correct")
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
        || (ends_confirmation && advisory_tail)
        || (starts_advisory && advisory_tail)
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

    host_scope && host_action
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
        || (lower.contains("path") && (lower.contains("show") || lower.contains("what is")));
    let asks_gpo = lower.contains("gpo")
        || lower.contains("group policy")
        || lower.contains("gpresult")
        || lower.contains("applied policy");
    let asks_certificates = lower.contains("cert")
        || lower.contains("ssl")
        || lower.contains("client cert")
        || lower.contains("expiring cert");
    let asks_integrity = lower.contains("integrity")
        || lower.contains("sfc")
        || lower.contains("dism")
        || lower.contains("corruption")
        || lower.contains("os health");
    let asks_user_accounts = lower.contains("user account")
        || lower.contains("local user")
        || lower.contains("local group")
        || lower.contains("get-localuser")
        || lower.contains("get-localgroup")
        || lower.contains("get-localgroupmember")
        || lower.contains("who is logged in")
        || lower.contains("who is logged on")
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
        || lower.contains("active sessions")
        || lower.contains("logon session")
        || lower.contains("net user")
        || lower.contains("net localgroup");
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
        || lower.contains("hardware failing");
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
        || lower.contains("wbiosrvc");
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
        || ((lower.contains("install") || lower.contains("installer"))
            && (lower.contains("fail")
                || lower.contains("failing")
                || lower.contains("broken")
                || lower.contains("stuck")
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
            && (lower.contains("backup drive")
                || lower.contains("backup disk")
                || lower.contains("configured")
                || lower.contains("schedule")
                || lower.contains("last backup")
                || lower.contains("backup health")
                || lower.contains("backup status")
                || lower.contains("broken")
                || lower.contains("failed")))
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
    let asks_ntp = lower.contains("ntp")
        || lower.contains("time sync")
        || lower.contains("clock sync")
        || lower.contains("w32tm")
        || lower.contains("clock drift")
        || lower.contains("system clock")
        || lower.contains("time server")
        || (lower.contains("time") && lower.contains("drift"))
        || (lower.contains("clock") && lower.contains("wrong"))
        || (lower.contains("time") && lower.contains("wrong"));
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
        || (lower.contains("boost") && lower.contains("disabled"));
    let asks_credentials = lower.contains("credential manager")
        || lower.contains("credential store")
        || lower.contains("saved password")
        || lower.contains("stored credential")
        || lower.contains("saved credential")
        || lower.contains("credential vault")
        || lower.contains("cmdkey")
        || (lower.contains("credential") && lower.contains("list"))
        || (lower.contains("password") && lower.contains("vault"))
        || (lower.contains("windows") && lower.contains("credential"));
    let asks_tpm = lower.contains("tpm")
        || lower.contains("secure boot")
        || lower.contains("secureboot")
        || lower.contains("trusted platform module")
        || lower.contains("firmware security")
        || lower.contains("uefi security")
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
        || (lower.contains("tcp")
            && (lower.contains("slow")
                || lower.contains("throughput")
                || lower.contains("performance")
                || lower.contains("config")));
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
                || lower.contains("saved")))
        || (lower.contains("wireless")
            && (lower.contains("profile") || lower.contains("saved") || lower.contains("audit")));
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
            && (lower.contains("nic") || lower.contains("adapter") || lower.contains("network")));
    let asks_snmp = lower.contains("snmp")
        || lower.contains("snmp agent")
        || lower.contains("snmp trap")
        || lower.contains("community string")
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
        || lower.contains("active user");
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
        || lower.contains("run at boot");
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
        || lower.contains("top memory")
        || lower.contains("top ram")
        || lower.contains("high memory")
        || lower.contains("resource-heavy processes")
        || lower.contains("heavy hitters")
        || (lower.contains("using the most")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("memory")))
        || (lower.contains("most cpu")
            || lower.contains("most ram")
            || lower.contains("most memory"))
        || (lower.contains("hitting")
            && (lower.contains("cpu") || lower.contains("ram") || lower.contains("disk")));
    let asks_toolchains = lower.contains("developer tools")
        || lower.contains("toolchains")
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
            && (lower.contains("reach") || lower.contains("access") || lower.contains("test")));
    let asks_thermal = lower.contains("thermal")
        || (lower.contains("throttle") && !lower.contains("gpu"))
        || lower.contains("overheating")
        || lower.contains("cpu temp");
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
            && (lower.contains("throttle")
                || lower.contains("bottleneck")
                || lower.contains("clock")
                || lower.contains("fan")
                || lower.contains("power draw")
                || lower.contains("frequency")
                || lower.contains("overheating")));
    let asks_hardware = lower.contains("cpu model")
        || lower.contains("ram size")
        || lower.contains("hardware spec")
        || (lower.contains("what hardware") && lower.contains("have"))
        || (lower.contains("gpu") && (lower.contains("what") || lower.contains("show")))
        || lower.contains("motherboard")
        || lower.contains("bios version");
    let asks_activation = lower.contains("activation")
        || lower.contains("slmgr")
        || lower.contains("license status")
        || lower.contains("is windows genuine");
    let asks_patch_history = lower.contains("patch history")
        || lower.contains("hotfix")
        || lower.contains("kb history")
        || lower.contains("installed updates");
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
            || contains_any(&lower, CODE_KEYWORDS));
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
        || lower.contains("last boot");
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
                || lower.contains("my machine")));
    let asks_security = lower.contains("antivirus")
        || lower.contains("defender")
        || lower.contains("virus protection")
        || lower.contains("malware")
        || lower.contains("windows security")
        || lower.contains("uac")
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
        || (lower.contains("restart") && lower.contains("pending"));
    let asks_disk_health = lower.contains("disk health")
        || lower.contains("drive health")
        || lower.contains("hard drive dying")
        || lower.contains("smart status")
        || lower.contains("drive failing")
        || lower.contains("drive fail")
        || (lower.contains("dying") && (lower.contains("drive") || lower.contains("disk")))
        || (lower.contains("healthy")
            && (lower.contains("drive")
                || lower.contains("disk")
                || lower.contains("ssd")
                || lower.contains("hdd")));
    let asks_battery = lower.contains("battery")
        || lower.contains("battery life")
        || lower.contains("battery health")
        || lower.contains("battery wear")
        || lower.contains("charge level")
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
        || lower.contains("background task");
    let asks_dev_conflicts = lower.contains("dev conflict")
        || lower.contains("environment conflict")
        || lower.contains("toolchain conflict")
        || lower.contains("version conflict")
        || lower.contains("path conflict")
        || lower.contains("duplicate path")
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
        || (lower.contains("where") && lower.contains("space") && lower.contains("go"));
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
        || lower.contains("slow")
        || lower.contains("lag")
        || lower.contains("sluggish")
        || lower.contains("hang")
        || lower.contains("unresponsive")
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
        || (lower.contains("dns") && (lower.contains("resolv") || lower.contains("working")));
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
        || (lower.contains("connection") && lower.contains("active"))
        || (lower.contains("connection") && lower.contains("open"));
    let asks_vpn = lower.contains("vpn")
        || lower.contains("virtual private network")
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
        || lower.contains("git aliases"))
        && !lower.contains("github");
    let asks_audit_policy = lower.contains("audit policy")
        || lower.contains("auditpol")
        || lower.contains("audit log")
        || lower.contains("what is being logged")
        || lower.contains("security audit")
        || lower.contains("logon event")
        || lower.contains("audit category")
        || lower.contains("event auditing");
    let asks_shares = lower.contains("smb share")
        || lower.contains("network share")
        || lower.contains("shared folder")
        || lower.contains("mapped drive")
        || lower.contains("mapped network drive")
        || lower.contains("get-smbshare")
        || lower.contains("what is shared")
        || lower.contains("what am i sharing")
        || lower.contains("smb session")
        || lower.contains("lanmanager")
        || lower.contains("netlanmanager")
        || lower.contains("smb1")
        || lower.contains("smb signing")
        || lower.contains("nfs export");
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
        || lower.contains("encryption status");
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
        || (lower.contains("paging") && lower.contains("file"));
    let asks_windows_features = (lower.contains("window") && lower.contains("feature"))
        || lower.contains("optional feature")
        || lower.contains("iis")
        || lower.contains("hyper-v")
        || (lower.contains("feature")
            && (lower.contains("install")
                || lower.contains("enabled")
                || lower.contains("turn on")));
    let asks_printers =
        lower.contains("printer") || lower.contains("print queue") || lower.contains("get-printer");
    let asks_winrm = lower.contains("winrm")
        || lower.contains("psremoting")
        || (lower.contains("ps") && lower.contains("remoting"))
        || (lower.contains("remote") && lower.contains("management") && !lower.contains("rdp"));
    let asks_network_stats = (lower.contains("network") && lower.contains("stat"))
        || (lower.contains("adapter") && lower.contains("stat"))
        || (lower.contains("nic") && lower.contains("stat"))
        || lower.contains("throughput")
        || lower.contains("dropped packet");
    let asks_udp_ports = lower.contains("udp port")
        || lower.contains("udp listener")
        || (lower.contains("udp")
            && (lower.contains("port") || lower.contains("listen") || lower.contains("open")));

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
    } else if asks_overclocker {
        Some("overclocker")
    } else if asks_traceroute {
        Some("traceroute")
    } else if asks_dhcp {
        Some("dhcp")
    } else if asks_mtu {
        Some("mtu")
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
    } else if asks_ipv6 {
        Some("ipv6")
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
    } else if asks_storage {
        Some("storage")
    } else if asks_gpo {
        Some("gpo")
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
    } else if asks_user_accounts {
        Some("user_accounts")
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
    } else if asks_env_doctor {
        Some("env_doctor")
    } else if asks_dns_servers {
        Some("dns_servers")
    } else if asks_lan_discovery {
        Some("lan_discovery")
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
    } else if asks_network_stats {
        Some("network_stats")
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

pub fn all_host_inspection_topics(user_input: &str) -> Vec<&'static str> {
    // All topic detectors in priority order — ordered so more specific topics come
    // before generic fallbacks (e.g. traceroute before network).
    let lower = user_input.to_lowercase();
    let mut topics: Vec<&'static str> = Vec::new();

    let detectors: &[(&str, fn(&str) -> bool)] = &[
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
                        || l.contains("overheating")))
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
                || l.contains("mdm enrollment")
                || l.contains("managed device")
                || l.contains("azure ad join")
                || (l.contains("enrolled") && l.contains("device"))
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
        }),
        ("security", |l| {
            l.contains("antivirus")
                || l.contains("defender")
                || l.contains("uac")
                || (l.contains("security") && !l.contains("git") && !l.contains("ssh"))
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
        }),
        ("thermal", |l| {
            l.contains("thermal") || l.contains("throttling") || l.contains("overheating")
        }),
        ("overclocker", |l| {
            l.contains("overclocker")
                || l.contains("gpu clock")
                || l.contains("nvidia stats")
                || l.contains("silicon health")
                || l.contains("mhz")
        }),
        ("activation", |l| {
            l.contains("activation") || l.contains("slmgr") || l.contains("license status")
        }),
        ("patch_history", |l| {
            l.contains("patch history") || l.contains("hotfix") || l.contains("kb history")
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
                || ((l.contains("install") || l.contains("installer"))
                    && (l.contains("fail")
                        || l.contains("failing")
                        || l.contains("broken")
                        || l.contains("stuck")
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
                || (l.contains("backup")
                    && (l.contains("drive")
                        || l.contains("running")
                        || l.contains("health")
                        || l.contains("status")
                        || l.contains("failed")))
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
                || l.contains("dpi")
                || l.contains("scaling")
        }),
        ("ntp", |l| {
            l.contains("ntp")
                || l.contains("time sync")
                || l.contains("clock sync")
                || l.contains("w32tm")
                || l.contains("clock drift")
                || (l.contains("time") && l.contains("drift"))
        }),
        ("cpu_power", |l| {
            l.contains("turbo boost")
                || l.contains("cpu frequency")
                || l.contains("cpu clock")
                || l.contains("cpu power")
                || l.contains("power plan")
                || (l.contains("cpu") && l.contains("slow"))
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
        }),
        ("tpm", |l| {
            l.contains("tpm")
                || l.contains("secure boot")
                || l.contains("trusted platform module")
                || l.contains("firmware security")
                || l.contains("uefi security")
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
                || (l.contains("tcp") && (l.contains("slow") || l.contains("throughput")))
        }),
        ("wlan_profiles", |l| {
            l.contains("saved wifi")
                || l.contains("wifi profile")
                || l.contains("wlan profile")
                || l.contains("wireless profile")
                || l.contains("saved network")
                || l.contains("netsh wlan")
                || (l.contains("wifi") && (l.contains("security") || l.contains("audit")))
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
        }),
        ("snmp", |l| {
            l.contains("snmp") || l.contains("community string") || l.contains("snmpd")
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
        }),
        ("disk_health", |l| {
            l.contains("disk health")
                || l.contains("drive health")
                || l.contains("smart status")
                || (l.contains("healthy")
                    && (l.contains("drive") || l.contains("disk") || l.contains("ssd")))
        }),
        ("battery", |l| l.contains("battery")),
        ("app_crashes", |l| {
            l.contains("application crash")
                || l.contains("application error")
                || l.contains("app hang")
                || l.contains("faulting application")
                || l.contains("wer report")
                || (l.contains("crash") && l.contains("program"))
                || (l.contains("crash")
                    && (l.contains("chrome")
                        || l.contains("edge")
                        || l.contains("firefox")
                        || l.contains("discord")
                        || l.contains("office")))
        }),
        ("recent_crashes", |l| {
            l.contains("crash") || l.contains("bsod") || l.contains("blue screen")
        }),
        ("scheduled_tasks", |l| {
            l.contains("scheduled task") || l.contains("task scheduler")
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
                    && (l.contains("running")
                        || l.contains("checkpoint")
                        || l.contains("snapshot")
                        || l.contains("switch")
                        || l.contains("ram")
                        || l.contains("memory")))
                || (l.contains("list") && l.contains("vm"))
        }),
        ("ip_config", |l| {
            l.contains("ipconfig") && (l.contains("all") || l.contains("detail"))
        }),
        ("dev_conflicts", |l| {
            l.contains("dev conflict")
                || l.contains("toolchain conflict")
                || l.contains("duplicate path")
        }),
        ("storage", |l| {
            l.contains("disk space")
                || l.contains("storage")
                || l.contains("drive capacity")
                || l.contains("cache size")
                || l.contains("i/o pressure")
                || l.contains("disk usage")
        }),
        ("hardware", |l| {
            l.contains("cpu model")
                || l.contains("ram size")
                || l.contains("hardware spec")
                || (l.contains("what hardware") && l.contains("have"))
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
                || l.contains("slow")
                || l.contains("lag")
                || l.contains("sluggish")
                || l.contains("hang")
                || l.contains("unresponsive")
        }),
        ("processes", |l| {
            l.contains("process")
                || l.contains("task manager")
                || l.contains("what is running")
                || l.contains("using my ram")
                || l.contains("hitting the disk")
                || l.contains("disk thrasher")
        }),
        ("services", |l| {
            l.contains("service") || l.contains("daemon") || l.contains("windows service")
        }),
        ("ports", |l| {
            l.contains("listening port")
                || l.contains("open port")
                || l.contains("what is on port")
                || l.contains("port 3000")
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
                || l.contains("netstat")
                || l.contains("open socket")
                || (l.contains("established") && l.contains("connection"))
        }),
        ("vpn", |l| {
            l.contains("vpn") || l.contains("virtual private network")
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
                        || l.contains("overheating")))
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
                || l.contains("what is installed")
                || l.contains("what's installed")
                || l.contains("winget list")
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
        }),
        ("audit_policy", |l| {
            l.contains("audit policy")
                || l.contains("auditpol")
                || l.contains("what is being logged")
                || l.contains("security audit")
                || l.contains("event auditing")
        }),
        ("shares", |l| {
            l.contains("smb share")
                || l.contains("network share")
                || l.contains("shared folder")
                || l.contains("mapped drive")
                || l.contains("smb1")
                || l.contains("nfs export")
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
                || l.contains("encryption status")
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
        }),
        ("windows_features", |l| {
            (l.contains("window") && l.contains("feature"))
                || l.contains("optional feature")
                || l.contains("iis")
                || l.contains("hyper-v")
                || (l.contains("feature") && (l.contains("install") || l.contains("enabled")))
        }),
        ("printers", |l| {
            l.contains("printer") || l.contains("print queue") || l.contains("get-printer")
        }),
        ("winrm", |l| {
            l.contains("winrm")
                || l.contains("psremoting")
                || (l.contains("remote") && l.contains("management") && !l.contains("rdp"))
        }),
        ("network_stats", |l| {
            (l.contains("network") && l.contains("stat"))
                || (l.contains("adapter") && l.contains("stat"))
                || l.contains("throughput")
                || l.contains("packet loss")
                || l.contains("dropped packet")
        }),
        ("startup_items", |l| {
            l.contains("startup") || l.contains("boot program") || l.contains("autorun")
        }),
        ("udp_ports", |l| {
            l.contains("udp port")
                || l.contains("udp listener")
                || (l.contains("udp") && l.contains("listening"))
        }),
        ("gpo", |l| {
            l.contains("gpo") || l.contains("group policy") || l.contains("gpresult")
        }),
        ("certificates", |l| {
            l.contains("cert") || l.contains("ssl") || l.contains("thumbprint")
        }),
        ("integrity", |l| {
            l.contains("integrity") || l.contains("sfc") || l.contains("dism")
        }),
        ("domain", |l| {
            l.contains("domain") || l.contains("workgroup") || l.contains("active directory")
        }),
        ("device_health", |l| {
            l.contains("device health")
                || l.contains("hardware error")
                || l.contains("yellow bang")
                || l.contains("malfunctioning")
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
            l.contains("session") || l.contains("who is logged") || l.contains("active login")
        }),
        ("hardware", |l| {
            l.contains("virtualization")
                || l.contains("hypervisor")
                || l.contains("vt-x")
                || l.contains("slat")
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
    } else if asks_script && !preferred_maintainer_workflow(user_input).is_some() {
        Some("script")
    } else if (asks_test || asks_lint || asks_fix)
        && !preferred_maintainer_workflow(user_input).is_some()
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
    let trimmed = user_input.trim().to_ascii_lowercase();

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

    let host_inspection_allowed = if is_coding_workflow && contains_any(&lower, CODE_KEYWORDS) {
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

    let direct_answer = if trimmed == "/about" || mentions_creator_question(&lower) {
        Some(DirectAnswerKind::About)
    } else if matches!(
        trimmed.as_str(),
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

/// Returns true when the user's query involves computation that must be exact —
/// checksums, financial math, statistics, date arithmetic, algorithmic verification, etc.
/// Used by the harness to inject a pre-turn nudge toward run_code instead of model memory.
pub fn needs_computation_sandbox(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    let hash_or_checksum = lower.contains("sha")
        || lower.contains("md5")
        || lower.contains("checksum")
        || lower.contains("crc")
        || lower.contains("hash")
        || lower.contains("fingerprint");
    let financial =
        (lower.contains("calculat") || lower.contains("compute") || lower.contains("what is"))
            && (lower.contains("percent")
                || lower.contains("%")
                || lower.contains("interest")
                || lower.contains("compound")
                || lower.contains("roi")
                || lower.contains("tax")
                || lower.contains("discount")
                || lower.contains("profit")
                || lower.contains("loss"));
    let statistics = lower.contains("standard deviation")
        || lower.contains("std dev")
        || lower.contains("mean of")
        || lower.contains("median of")
        || lower.contains("average of")
        || lower.contains("variance")
        || lower.contains("regression")
        || lower.contains("correlation");
    let date_math = (lower.contains("how many days")
        || lower.contains("days between")
        || lower.contains("days until")
        || lower.contains("days since")
        || lower.contains("unix timestamp")
        || lower.contains("epoch")
        || lower.contains("time zone")
        || lower.contains("timezone"))
        && (lower.contains("date")
            || lower.contains("day")
            || lower.contains("timestamp")
            || lower.contains("time"));
    let algorithmic = lower.contains("is prime")
        || lower.contains("prime number")
        || lower.contains("factori")
        || lower.contains("fibonacci")
        || lower.contains("factorial")
        || lower.contains("sort this")
        || lower.contains("verify this algorithm")
        || lower.contains("run this code")
        || lower.contains("execute this");
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
            || lower.contains("miles")
            || lower.contains("pounds")
            || lower.contains("kilograms"));
    hash_or_checksum || financial || statistics || date_math || algorithmic || unit_conversion
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
