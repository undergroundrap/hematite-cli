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
            "traceroute",
            "tracert",
            "dns cache",
            "arp table",
            "arp cache",
            "route table",
            "routing table",
            "default gateway",
            "next hop",
            "power plan",
            "power settings",
            "uptime",
            "reboot",
            "health",
            "report",
            "bitlocker",
            "rdp",
            "remote desktop",
            "vss",
            "shadow copy",
            "shadow copies",
            "pagefile",
            "virtual memory",
            "swap",
            "windows feature",
            "optional feature",
            "printer",
            "print queue",
            "winrm",
            "psremoting",
            "network stats",
            "adapter stats",
            "udp listening",
            "udp port",
            "session",
            "logon",
            "login",
            "virtualization",
            "hypervisor",
            "vt-x",
            "slat",
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
    let asks_path = lower.contains("path entries") || lower.contains("raw path") || (lower.contains("path") && (lower.contains("show") || lower.contains("what is")));
    let asks_gpo = lower.contains("gpo") || lower.contains("group policy") || lower.contains("gpresult") || lower.contains("applied policy");
    let asks_certificates = lower.contains("cert") || lower.contains("ssl") || lower.contains("client cert") || lower.contains("expiring cert");
    let asks_integrity = lower.contains("integrity") || lower.contains("sfc") || lower.contains("dism") || lower.contains("corruption") || lower.contains("os health");
    let asks_domain = lower.contains("domain") || lower.contains("active directory") || lower.contains("ad join") || lower.contains("workgroup") || lower.contains("netbios");
    let asks_device_health = lower.contains("device health") || lower.contains("hardware error") || lower.contains("malfunctioning") || lower.contains("yellow bang") || lower.contains("hardware failing");
    let asks_drivers = lower.contains("driver") || lower.contains("kmod") || lower.contains("kernel module");
    let asks_peripherals = lower.contains("peripheral") || lower.contains("usb") || lower.contains("keyboard") || lower.contains("mouse") || lower.contains("pointer") || lower.contains("monitor") || lower.contains("input device") || lower.contains("connected hardware");
    let asks_sessions = lower.contains("session") || lower.contains("login") || lower.contains("user account") || lower.contains("who is on") || lower.contains("active user");
    let asks_virtualization = lower.contains("virtualization") || lower.contains("hypervisor") || lower.contains("vt-x") || lower.contains("slat") || lower.contains("v-p") || lower.contains("nested virt");
    let asks_startup = lower.contains("startup") || lower.contains("boot program") || lower.contains("autorun") || lower.contains("run at boot");
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
    let asks_traceroute = lower.contains("traceroute")
        || lower.contains("tracert")
        || lower.contains("tracepath")
        || lower.contains("trace route")
        || lower.contains("trace the path")
        || lower.contains("network path")
        || lower.contains("how many hops")
        || lower.contains("where does traffic go")
        || (lower.contains("trace") && lower.contains("hop"))
        || (lower.contains("route") && lower.contains("traffic"));
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
        || (lower.contains("arp") && (lower.contains("who") || lower.contains("entry") || lower.contains("entries")));
    let asks_route_table = lower.contains("route print")
        || lower.contains("route table")
        || lower.contains("routing table")
        || lower.contains("get-netroute")
        || lower.contains("default gateway")
        || lower.contains("network routes")
        || lower.contains("ip route")
        || lower.contains("next hop")
        || (lower.contains("route") && (lower.contains("table") || lower.contains("entry") || lower.contains("entries")));
    let asks_env = (lower.contains("environment variable") || lower.contains("env var")
        || lower.contains("env vars") || lower.contains("show env") || lower.contains("list env"))
        && !lower.contains("env doctor");
    let asks_hosts_file = lower.contains("hosts file")
        || lower.contains("/etc/hosts")
        || lower.contains("etc/hosts")
        || lower.contains("hosts entry")
        || lower.contains("hosts entries")
        || (lower.contains("hosts") && (lower.contains("redirect") || lower.contains("block") || lower.contains("loopback")));
    let asks_docker = lower.contains("docker")
        || lower.contains("container")
        || lower.contains("docker compose")
        || lower.contains("docker ps")
        || lower.contains("running container");
    let asks_wsl = lower.contains("wsl")
        || lower.contains("windows subsystem")
        || lower.contains("linux distro")
        || lower.contains("ubuntu on windows")
        || (lower.contains("subsystem") && lower.contains("linux"));
    let asks_ssh = (lower.contains("ssh") && !lower.contains("ssh key") && !lower.contains("git"))
        || lower.contains("sshd")
        || lower.contains("ssh config")
        || lower.contains("ssh server")
        || lower.contains("ssh client")
        || lower.contains("known_hosts")
        || lower.contains("authorized_keys")
        || lower.contains("ssh key")
        || (lower.contains("ssh") && (lower.contains("running") || lower.contains("service") || lower.contains("port 22")));
    let asks_installed_software = lower.contains("installed software")
        || lower.contains("installed program")
        || lower.contains("installed app")
        || lower.contains("installed package")
        || lower.contains("what is installed")
        || lower.contains("what's installed")
        || lower.contains("winget list")
        || lower.contains("list programs")
        || (lower.contains("installed") && (lower.contains("on this machine") || lower.contains("on my machine") || lower.contains("on my pc")));
    let asks_databases = lower.contains("postgres") || lower.contains("postgresql")
        || lower.contains("mysql") || lower.contains("mariadb")
        || lower.contains("mongodb") || lower.contains("mongo")
        || lower.contains("redis") || lower.contains("sql server") || lower.contains("mssql")
        || lower.contains("sqlite") || lower.contains("elasticsearch") || lower.contains("cassandra")
        || lower.contains("couchdb")
        || (lower.contains("database") && (lower.contains("running") || lower.contains("service")
            || lower.contains("installed") || lower.contains("up") || lower.contains("local")))
        || lower.contains("db service") || lower.contains("database server")
        || (lower.contains("is") && lower.contains("running") && (lower.contains("db") || lower.contains("database")));
    let asks_git_config = (lower.contains("git config") || lower.contains("git configuration")
        || lower.contains("git global")
        || (lower.contains("git") && lower.contains("user.name"))
        || (lower.contains("git") && lower.contains("user.email"))
        || (lower.contains("git") && lower.contains("signing"))
        || (lower.contains("git") && lower.contains("credential"))
        || lower.contains("git aliases"))
        && !lower.contains("github");
    let asks_user_accounts = lower.contains("local user")
        || lower.contains("user account")
        || lower.contains("who is logged in")
        || lower.contains("who is logged on")
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
        || lower.contains("get-localuser");
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
        && !lower.contains("dns cache");
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
        || (lower.contains("feature") && (lower.contains("install") || lower.contains("enabled") || lower.contains("turn on")));
    let asks_printers = lower.contains("printer")
        || lower.contains("print queue")
        || lower.contains("get-printer");
    let asks_winrm = lower.contains("winrm")
        || lower.contains("psremoting")
        || (lower.contains("ps") && lower.contains("remoting"))
        || (lower.contains("remote") && lower.contains("management") && !lower.contains("rdp"));
    let asks_network_stats = (lower.contains("network") && lower.contains("stat"))
        || (lower.contains("adapter") && lower.contains("stat"))
        || (lower.contains("nic") && lower.contains("stat"))
        || lower.contains("throughput")
        || lower.contains("packet loss")
        || lower.contains("dropped packet");
    let asks_udp_ports = lower.contains("udp port")
        || lower.contains("udp listener")
        || (lower.contains("udp") && (lower.contains("port") || lower.contains("listen") || lower.contains("open")));

    if asks_fix_plan {
        Some("fix_plan")
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
    } else if asks_sessions {
        Some("sessions")
    } else if asks_virtualization {
        Some("hardware")
    } else if asks_startup {
        Some("startup_items")
    }
    else if asks_bitlocker {
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
    } else if asks_network_stats {
        Some("network_stats")
    } else if asks_udp_ports {
        Some("udp_ports")
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
    } else if asks_dns_servers {
        Some("dns_servers")
    } else if asks_user_accounts {
        Some("user_accounts")
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
    } else if asks_traceroute {
        Some("traceroute")
    } else if asks_dns_cache {
        Some("dns_cache")
    } else if asks_arp {
        Some("arp")
    } else if asks_route_table {
        Some("route_table")
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
    } else if asks_shares {
        Some("shares")
    } else if asks_hosts_file {
        Some("hosts_file")
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

/// Returns all distinct inspect_host topics detected in a user prompt.
/// Used by the harness to pre-run multiple topics when the user asks for several at once,
/// so the model receives all results and only needs to synthesize rather than orchestrate.
pub fn all_host_inspection_topics(user_input: &str) -> Vec<&'static str> {
    // All topic detectors in priority order — ordered so more specific topics come
    // before generic fallbacks (e.g. traceroute before network).
    let detectors: &[(&str, fn(&str) -> bool)] = &[
        ("fix_plan",        |l| l.contains("fix") && (l.contains("cargo") || l.contains("port ") || l.contains("lm studio") || l.contains("toolchain"))),
        ("updates",         |l| l.contains("up to date") || l.contains("windows update") || l.contains("pending update") || l.contains("update available")),
        ("security",        |l| l.contains("antivirus") || l.contains("defender") || l.contains("uac") || (l.contains("security") && !l.contains("git") && !l.contains("ssh"))),
        ("pending_reboot",  |l| l.contains("pending reboot") || l.contains("pending restart") || l.contains("need to restart") || l.contains("reboot required")),
        ("disk_health",     |l| l.contains("disk health") || l.contains("drive health") || l.contains("smart status") || (l.contains("healthy") && (l.contains("drive") || l.contains("disk") || l.contains("ssd")))),
        ("battery",         |l| l.contains("battery")),
        ("recent_crashes",  |l| l.contains("crash") || l.contains("bsod") || l.contains("blue screen")),
        ("scheduled_tasks", |l| l.contains("scheduled task") || l.contains("task scheduler")),
        ("dev_conflicts",   |l| l.contains("dev conflict") || l.contains("toolchain conflict") || l.contains("duplicate path")),
        ("storage",         |l| l.contains("disk space") || l.contains("storage") || l.contains("drive capacity") || l.contains("cache size")),
        ("hardware",        |l| l.contains("cpu model") || l.contains("ram size") || l.contains("hardware spec") || (l.contains("what hardware") && l.contains("have"))),
        ("health_report",   |l| l.contains("health report") || l.contains("system health")),
        ("resource_load",   |l| l.contains("resource load") || l.contains("cpu load") || l.contains("ram %") || l.contains("cpu %") || l.contains("performance")),
        ("processes",       |l| l.contains("process") || l.contains("task manager") || l.contains("what is running") || l.contains("using my ram") || l.contains("hitting the disk") || l.contains("disk thrasher")),
        ("services",        |l| l.contains("service") || l.contains("daemon") || l.contains("windows service")),
        ("ports",           |l| l.contains("listening port") || l.contains("open port") || l.contains("what is on port") || l.contains("port 3000")),
        ("traceroute",      |l| l.contains("traceroute") || l.contains("tracert") || l.contains("trace route") || l.contains("trace the path") || l.contains("network path") || l.contains("how many hops") || (l.contains("trace") && l.contains("hop"))),
        ("dns_cache",       |l| l.contains("dns cache") || l.contains("cached dns") || l.contains("displaydns") || (l.contains("dns") && l.contains("cached"))),
        ("arp",             |l| l.contains("arp table") || l.contains("arp cache") || l.contains("mac address") || l.contains("ip to mac") || l.contains("arp -")),
        ("route_table",     |l| l.contains("route table") || l.contains("routing table") || l.contains("route print") || l.contains("network route") || l.contains("next hop")),
        ("connectivity",    |l| l.contains("internet") || l.contains("am i connected") || l.contains("ping google") || l.contains("internet access") || l.contains("no internet")),
        ("wifi",            |l| l.contains("wi-fi") || l.contains("wifi") || l.contains("wireless") || l.contains("ssid") || l.contains("signal strength")),
        ("connections",     |l| l.contains("tcp connection") || l.contains("active connection") || l.contains("netstat") || l.contains("open socket")),
        ("vpn",             |l| l.contains("vpn") || l.contains("virtual private network")),
        ("proxy",           |l| l.contains("proxy setting") || l.contains("system proxy") || l.contains("winhttp proxy")),
        ("firewall_rules",  |l| (l.contains("firewall") && (l.contains("rule") || l.contains("inbound") || l.contains("outbound"))) || l.contains("firewall rule")),
        ("network",         |l| l.contains("network adapter") || l.contains("ip address") || l.contains("ipconfig") || l.contains("gateway") || l.contains("subnet")),
        ("env_doctor",      |l| l.contains("env doctor") || l.contains("environment doctor") || l.contains("package manager") || l.contains("path drift")),
        ("os_config",       |l| l.contains("power plan") || l.contains("uptime") || l.contains("boot time") || l.contains("last boot")),
        ("path",               |l| l.contains("path entries") || l.contains("raw path")),
        ("toolchains",         |l| l.contains("developer tools") || l.contains("toolchains") || (l.contains("installed") && l.contains("version"))),
        ("docker",             |l| l.contains("docker") || l.contains("container") || l.contains("running container")),
        ("wsl",                |l| l.contains("wsl") || l.contains("windows subsystem") || (l.contains("subsystem") && l.contains("linux"))),
        ("ssh",                |l| l.contains("ssh") || l.contains("sshd") || l.contains("known_hosts") || l.contains("authorized_keys")),
        ("git_config",         |l| (l.contains("git config") || l.contains("git global") || l.contains("git aliases")) && !l.contains("github")),
        ("installed_software", |l| l.contains("installed software") || l.contains("installed program") || l.contains("what is installed") || l.contains("what's installed") || l.contains("winget list")),
        ("env",                |l| (l.contains("environment variable") || l.contains("env var") || l.contains("env vars")) && !l.contains("env doctor")),
        ("hosts_file",         |l| l.contains("hosts file") || l.contains("/etc/hosts") || l.contains("hosts entry")),
        ("databases",          |l| l.contains("postgres") || l.contains("mysql") || l.contains("mariadb") || l.contains("mongodb") || l.contains("redis") || l.contains("sqlite") || l.contains("sql server") || l.contains("elasticsearch") || (l.contains("database") && (l.contains("running") || l.contains("service")))),
        ("user_accounts",      |l| l.contains("local user") || l.contains("user account") || l.contains("who is logged") || l.contains("admin group") || l.contains("local admin") || l.contains("active sessions") || l.contains("running as admin")),
        ("audit_policy",       |l| l.contains("audit policy") || l.contains("auditpol") || l.contains("what is being logged") || l.contains("security audit") || l.contains("event auditing")),
        ("shares",             |l| l.contains("smb share") || l.contains("network share") || l.contains("shared folder") || l.contains("mapped drive") || l.contains("smb1") || l.contains("nfs export")),
        ("dns_servers",        |l| (l.contains("dns server") || l.contains("dns resolver") || l.contains("nameserver") || l.contains("which dns") || l.contains("dns over https") || l.contains("configured dns")) && !l.contains("dns cache")),
        ("bitlocker",        |l| l.contains("bitlocker") || (l.contains("drive") && l.contains("encrypt")) || (l.contains("disk") && l.contains("encrypt")) || l.contains("encryption status")),
        ("rdp",              |l| l.contains("rdp") || l.contains("remote desktop") || (l.contains("remote") && l.contains("access") && !l.contains("git"))),
        ("shadow_copies",    |l| l.contains("shadow copy") || l.contains("shadow copies") || l.contains("vss") || l.contains("snapshot") || l.contains("restore point")),
        ("pagefile",         |l| l.contains("pagefile") || l.contains("page file") || l.contains("virtual memory") || l.contains("swap file")),
        ("windows_features", |l| (l.contains("window") && l.contains("feature")) || l.contains("optional feature") || l.contains("iis") || l.contains("hyper-v") || (l.contains("feature") && (l.contains("install") || l.contains("enabled")))),
        ("printers",         |l| l.contains("printer") || l.contains("print queue") || l.contains("get-printer")),
        ("winrm",            |l| l.contains("winrm") || l.contains("psremoting") || (l.contains("remote") && l.contains("management") && !l.contains("rdp"))),
        ("network_stats",    |l| (l.contains("network") && l.contains("stat")) || (l.contains("adapter") && l.contains("stat")) || l.contains("throughput") || l.contains("packet loss") || l.contains("dropped packet")),
        ("startup_items",    |l| l.contains("startup") || l.contains("boot program") || l.contains("autorun")),
        ("udp_ports",        |l| l.contains("udp port") || l.contains("udp listener") || (l.contains("udp") && l.contains("listening"))),
        ("gpo",              |l| l.contains("gpo") || l.contains("group policy") || l.contains("gpresult")),
        ("certificates",     |l| l.contains("cert") || l.contains("ssl") || l.contains("thumbprint")),
        ("integrity",        |l| l.contains("integrity") || l.contains("sfc") || l.contains("dism")),
        ("domain",           |l| l.contains("domain") || l.contains("workgroup") || l.contains("netbios") || l.contains("active directory")),
        ("device_health",    |l| l.contains("device health") || l.contains("hardware error") || l.contains("yellow bang") || l.contains("malfunctioning")),
        ("drivers",          |l| l.contains("driver") || l.contains("system driver")),
        ("peripherals",      |l| l.contains("peripheral") || l.contains("usb") || l.contains("keyboard") || l.contains("mouse") || l.contains("monitor")),
        ("sessions",         |l| l.contains("session") || l.contains("who is logged") || l.contains("active login")),
        ("hardware",         |l| l.contains("virtualization") || l.contains("hypervisor") || l.contains("vt-x") || l.contains("slat")),
    ];

    let lower = user_input.to_lowercase();
    let mut topics: Vec<&'static str> = Vec::new();
    for (topic, check) in detectors {
        if check(&lower) && !topics.contains(topic) {
            topics.push(topic);
        }
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
