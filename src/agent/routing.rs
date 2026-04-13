use super::conversation::WorkflowMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryIntentClass {
    ProductTruth,
    RuntimeDiagnosis,
    RepoArchitecture,
    Toolchain,
    Capability,
    Implementation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectAnswerKind {
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
    ArchitectSessionResetPlan,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct QueryIntent {
    pub(crate) primary_class: QueryIntentClass,
    pub(crate) direct_answer: Option<DirectAnswerKind>,
    pub(crate) grounded_trace_mode: bool,
    pub(crate) capability_mode: bool,
    pub(crate) capability_needs_repo: bool,
    pub(crate) toolchain_mode: bool,
    pub(crate) host_inspection_mode: bool,
    pub(crate) maintainer_workflow_mode: bool,
    pub(crate) workspace_workflow_mode: bool,
    pub(crate) architecture_overview_mode: bool,
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
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

fn mentions_capability_question(lower: &str) -> bool {
    contains_any(
        lower,
        &[
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

fn mentions_host_inspection_question(lower: &str) -> bool {
    let host_scope = contains_any(
        lower,
        &[
            "path",
            "package manager",
            "package managers",
            "env doctor",
            "environment doctor",
            "pip",
            "winget",
            "choco",
            "scoop",
            "network",
            "adapter",
            "dns",
            "gateway",
            "ip address",
            "ipconfig",
            "wifi",
            "ethernet",
            "service",
            "services",
            "daemon",
            "startup type",
            "process",
            "processes",
            "task manager",
            "ram",
            "cpu",
            "memory",
            "developer tools",
            "toolchains",
            "installed",
            "desktop",
            "downloads",
            "folder",
            "directory",
            "local development",
            "machine",
            "computer",
            "firewall",
            "vpn",
            "proxy",
            "internet",
            "online",
            "connectivity",
            "ssid",
            "wireless",
            "tcp connection",
            "active connection",
            "power plan",
            "power settings",
            "uptime",
            "reboot",
            "health",
            "report",
        ],
    );
    let host_action = contains_any(
        lower,
        &[
            "inspect",
            "count",
            "tell me",
            "summarize",
            "how big",
            "biggest",
            "versions",
            "duplicate",
            "missing",
            "ready",
            "fix",
            "repair",
            "resolve",
            "troubleshoot",
        ],
    );

    host_scope && host_action
}

pub(crate) fn preferred_host_inspection_topic(user_input: &str) -> Option<&'static str> {
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
    let asks_path = lower.contains("path");
    let asks_env_doctor = lower.contains("env doctor")
        || lower.contains("environment doctor")
        || lower.contains("package manager")
        || lower.contains("package managers")
        || lower.contains("shims")
        || lower.contains("path drift")
        || (lower.contains("dev machine") && lower.contains("off"))
        || (lower.contains("environment") && lower.contains("sane"));
    let asks_network = lower.contains("network")
        || lower.contains("adapter")
        || lower.contains("dns")
        || lower.contains("gateway")
        || lower.contains("ip address")
        || lower.contains("ipconfig")
        || lower.contains("wifi")
        || lower.contains("ethernet")
        || lower.contains("subnet");
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
        || lower.contains("heavy hitters");
    let asks_toolchains = lower.contains("developer tools")
        || lower.contains("toolchains")
        || (lower.contains("installed") && lower.contains("version"))
        || (lower.contains("detect") && lower.contains("version"));
    let asks_ports = lower.contains("listening on port")
        || lower.contains("listening port")
        || lower.contains("open port")
        || lower.contains("port 3000")
        || lower.contains("port ")
        || lower.contains("listening on ")
        || lower.contains("exposed")
        || lower.contains("what is listening");
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
    let asks_broad_readiness = lower.contains("local development")
        || lower.contains("ready for local development")
        || (lower.contains("machine") && lower.contains("ready"))
        || (lower.contains("computer") && lower.contains("ready"));
    let asks_os_config = lower.contains("firewall")
        || lower.contains("power plan")
        || lower.contains("power settings")
        || lower.contains("powercfg")
        || lower.contains("uptime")
        || lower.contains("reboot")
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
        || (lower.contains("update") && (lower.contains("my pc") || lower.contains("my computer") || lower.contains("my machine")));
    let asks_security = lower.contains("antivirus")
        || lower.contains("defender")
        || lower.contains("virus protection")
        || lower.contains("malware")
        || lower.contains("windows security")
        || lower.contains("uac")
        || lower.contains("windows activated")
        || lower.contains("activation status")
        || (lower.contains("protected") && (lower.contains("pc") || lower.contains("computer")))
        || (lower.contains("security") && !lower.contains("git") && !lower.contains("ssh") && !lower.contains("token"));
    let asks_pending_reboot = lower.contains("need to restart")
        || lower.contains("need to reboot")
        || lower.contains("requires restart")
        || lower.contains("requires a reboot")
        || lower.contains("reboot required")
        || lower.contains("restart required")
        || lower.contains("pending restart")
        || lower.contains("pending reboot")
        || (lower.contains("restart") && (lower.contains("waiting") || lower.contains("queued") || lower.contains("required")))
        || (lower.contains("reboot") && lower.contains("required"));
    let asks_disk_health = lower.contains("disk health")
        || lower.contains("drive health")
        || lower.contains("hard drive dying")
        || lower.contains("smart status")
        || lower.contains("drive failing")
        || lower.contains("drive fail")
        || (lower.contains("dying") && (lower.contains("drive") || lower.contains("disk")))
        || (lower.contains("healthy") && (lower.contains("drive") || lower.contains("disk") || lower.contains("ssd") || lower.contains("hdd")));
    let asks_battery = lower.contains("battery")
        || lower.contains("battery life")
        || lower.contains("battery health")
        || lower.contains("battery wear")
        || lower.contains("charge level")
        || lower.contains("how long until")
        || (lower.contains("dying") && lower.contains("batter"));
    let asks_recent_crashes = lower.contains("crash")
        || lower.contains("bsod")
        || lower.contains("blue screen")
        || lower.contains("why did my pc restart")
        || lower.contains("unexpected restart")
        || lower.contains("sudden restart")
        || lower.contains("kernel panic")
        || lower.contains("app crash")
        || (lower.contains("restart") && lower.contains("itself"))
        || (lower.contains("restart") && lower.contains("by itself"));
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
    let asks_firewall_rules = (lower.contains("firewall") && (lower.contains("rule")
        || lower.contains("block")
        || lower.contains("allow")
        || lower.contains("inbound")
        || lower.contains("outbound")))
        || lower.contains("blocked port")
        || lower.contains("firewall rule");

    if asks_fix_plan {
        Some("fix_plan")
    } else if asks_updates {
        Some("updates")
    } else if asks_security {
        Some("security")
    } else if asks_pending_reboot {
        Some("pending_reboot")
    } else if asks_disk_health {
        Some("disk_health")
    } else if asks_battery {
        Some("battery")
    } else if asks_recent_crashes {
        Some("recent_crashes")
    } else if asks_scheduled_tasks {
        Some("scheduled_tasks")
    } else if asks_dev_conflicts {
        Some("dev_conflicts")
    } else if (asks_path && asks_toolchains)
        || (mentions_host_inspection_question(&lower) && asks_broad_readiness)
    {
        Some("summary")
    } else if asks_env_doctor {
        Some("env_doctor")
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
    } else if asks_network {
        Some("network")
    } else if asks_services {
        Some("services")
    } else if asks_processes {
        Some("processes")
    } else if asks_ports {
        Some("ports")
    } else if asks_repo_doctor {
        Some("repo_doctor")
    } else if lower.contains("desktop") {
        Some("desktop")
    } else if lower.contains("downloads") {
        Some("downloads")
    } else if asks_path {
        Some("path")
    } else if asks_toolchains {
        Some("toolchains")
    } else if asks_os_config {
        Some("os_config")
    } else if asks_resource_load {
        Some("resource_load")
    } else if asks_health_report {
        Some("health_report")
    } else if asks_directory {
        Some("directory")
    } else if mentions_host_inspection_question(&lower) {
        Some("summary")
    } else {
        None
    }
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

pub(crate) fn preferred_workspace_workflow(user_input: &str) -> Option<&'static str> {
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
    let asks_script = contains_any(
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

    if asks_build
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
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn classify_query_intent(workflow_mode: WorkflowMode, user_input: &str) -> QueryIntent {
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
    let host_inspection_mode = preferred_host_inspection_topic(&lower).is_some();
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
        "who are you" | "who are you?" | "what are you" | "what are you?"
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
    } else {
        None
    };

    let primary_class = if direct_answer.is_some()
        || mentions_stable_product_surface(&lower)
        || mentions_product_truth_routing(&lower)
    {
        QueryIntentClass::ProductTruth
    } else if architecture_overview_mode {
        QueryIntentClass::RepoArchitecture
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
        maintainer_workflow_mode,
        workspace_workflow_mode,
        architecture_overview_mode,
    }
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
}
