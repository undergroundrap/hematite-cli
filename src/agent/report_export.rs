use serde_json::json;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

const REPORT_TOPICS: &[(&str, &str)] = &[
    ("health_report", "System Health"),
    ("hardware", "Hardware"),
    ("storage", "Storage"),
    ("network", "Network"),
    ("security", "Security"),
    ("toolchains", "Developer Toolchains"),
];

pub fn report_topics() -> &'static [(&'static str, &'static str)] {
    REPORT_TOPICS
}

/// IT-first-look triage topics (health, security, connectivity, identity, updates).
const TRIAGE_TOPICS: &[(&str, &str)] = &[
    ("health_report", "System Health"),
    ("security", "Security Posture"),
    ("connectivity", "Connectivity"),
    ("identity_auth", "Identity & Auth (M365/AAD)"),
    ("updates", "Windows Updates"),
];

pub fn triage_topics_for_preset(preset: &str) -> &'static [(&'static str, &'static str)] {
    match preset {
        "network" => &[
            ("connectivity", "Connectivity"),
            ("wifi", "Wi-Fi"),
            ("latency", "Latency"),
            ("dns_servers", "DNS Servers"),
            ("vpn", "VPN"),
            ("proxy", "Proxy"),
            ("connections", "Active Connections"),
        ],
        "security" => &[
            ("security", "Security Posture"),
            ("bitlocker", "BitLocker"),
            ("tpm", "TPM / Secure Boot"),
            ("local_security_policy", "Local Security Policy"),
            ("shares", "SMB Shares"),
            ("print_spooler", "Print Spooler"),
        ],
        "performance" => &[
            ("resource_load", "Resource Load"),
            ("thermal", "Thermal"),
            ("cpu_power", "CPU Power"),
            ("processes", "Top Processes"),
            ("pagefile", "Page File"),
            ("startup_items", "Startup Items"),
        ],
        "storage" => &[
            ("storage", "Storage"),
            ("disk_health", "Disk Health"),
            ("shadow_copies", "Shadow Copies"),
            ("storage_spaces", "Storage Spaces"),
            ("bitlocker", "BitLocker"),
        ],
        "apps" => &[
            ("browser_health", "Browser Health"),
            ("outlook", "Outlook"),
            ("teams", "Teams"),
            ("installer_health", "Installer Health"),
            ("onedrive", "OneDrive"),
        ],
        _ => TRIAGE_TOPICS,
    }
}

fn triage_preset_title(preset: &str) -> &'static str {
    match preset {
        "network" => "Hematite Network Triage Report",
        "security" => "Hematite Security Triage Report",
        "performance" => "Hematite Performance Triage Report",
        "storage" => "Hematite Storage Triage Report",
        "apps" => "Hematite App Health Triage Report",
        _ => "Hematite IT Triage Report",
    }
}

/// Map a plain-English issue description to the most relevant inspect_host topics.
fn topics_for_issue(issue: &str) -> Vec<(&'static str, &'static str)> {
    let lower = issue.to_ascii_lowercase();
    let mut seen = std::collections::HashSet::new();
    let mut topics: Vec<(&'static str, &'static str)> = Vec::new();

    macro_rules! add_if {
        ($keywords:expr, $pairs:expr) => {
            if $keywords.iter().any(|k: &&str| lower.contains(k)) {
                for &pair in $pairs {
                    if seen.insert(pair.0) {
                        topics.push(pair);
                    }
                }
            }
        };
    }

    add_if!(
        &[
            "slow",
            "lag",
            "freeze",
            "hang",
            "sluggish",
            "unresponsive",
            "performance",
            "high cpu",
            "high ram",
            "high memory",
            "locking up"
        ],
        &[
            ("resource_load", "Resource Load"),
            ("thermal", "Thermal"),
            ("cpu_power", "CPU Power"),
            ("pagefile", "Page File"),
            ("startup_items", "Startup Items")
        ]
    );
    add_if!(
        &[
            "internet",
            "network",
            "wifi",
            "wi-fi",
            "wireless",
            "offline",
            "no web",
            "can't browse",
            "ping fails",
            "no connection",
            "can't connect"
        ],
        &[
            ("connectivity", "Connectivity"),
            ("wifi", "Wi-Fi"),
            ("latency", "Latency"),
            ("dns_servers", "DNS Servers")
        ]
    );
    add_if!(
        &["dns ", "dns:", "name resolution", "can't resolve"],
        &[
            ("dns_servers", "DNS Servers"),
            ("connectivity", "Connectivity")
        ]
    );
    add_if!(
        &["vpn ", "vpn:", "tunnel", "remote access"],
        &[
            ("vpn", "VPN"),
            ("connectivity", "Connectivity"),
            ("proxy", "Proxy")
        ]
    );
    add_if!(
        &[
            "disk full",
            "out of space",
            "low disk",
            "disk space",
            "drive full",
            "storage full",
            "no space"
        ],
        &[
            ("storage", "Storage"),
            ("disk_health", "Disk Health"),
            ("shadow_copies", "Shadow Copies")
        ]
    );
    add_if!(
        &[
            "disk fail",
            "drive fail",
            "smart error",
            "disk error",
            "bad sector",
            "drive health"
        ],
        &[("disk_health", "Disk Health"), ("storage", "Storage")]
    );
    add_if!(
        &[
            "slow boot",
            "boot slow",
            "slow startup",
            "startup slow",
            "takes forever to boot"
        ],
        &[
            ("startup_items", "Startup Items"),
            ("services", "Services"),
            ("disk_health", "Disk Health")
        ]
    );
    add_if!(
        &[
            "crash",
            "bsod",
            "blue screen",
            "unexpected restart",
            "unexpected shutdown",
            "kernel panic",
            "stop error"
        ],
        &[
            ("recent_crashes", "Crash History"),
            ("log_check", "Event Log"),
            ("thermal", "Thermal"),
            ("disk_health", "Disk Health")
        ]
    );
    add_if!(
        &[
            "app crash",
            "application crash",
            "program crash",
            "program not opening",
            "app not starting",
            "not responding",
            "application error"
        ],
        &[
            ("app_crashes", "Application Crashes"),
            ("log_check", "Event Log")
        ]
    );
    add_if!(
        &[
            "update",
            "windows update",
            "patch",
            "stuck on update",
            "update fail"
        ],
        &[
            ("updates", "Windows Updates"),
            ("pending_reboot", "Pending Reboot"),
            ("services", "Services")
        ]
    );
    add_if!(
        &[
            "virus",
            "malware",
            "hacked",
            "suspicious",
            "threat",
            "infected",
            "ransomware"
        ],
        &[
            ("security", "Security Posture"),
            ("defender_quarantine", "Defender Quarantine"),
            ("log_check", "Event Log")
        ]
    );
    add_if!(
        &[
            "firewall",
            "blocked port",
            "blocked connection",
            "port block"
        ],
        &[
            ("security", "Security Posture"),
            ("firewall_rules", "Firewall Rules")
        ]
    );
    add_if!(
        &[
            "printer",
            "printing",
            "print queue",
            "can't print",
            "print fail"
        ],
        &[
            ("printers", "Printers"),
            ("print_spooler", "Print Spooler"),
            ("drivers", "Drivers")
        ]
    );
    add_if!(
        &[
            "sound",
            "audio",
            "speaker",
            "no sound",
            "headset",
            "mic",
            "microphone",
            "crackling",
            "audio fail"
        ],
        &[("audio", "Audio")]
    );
    add_if!(
        &[
            "bluetooth",
            "headphones",
            "airpods",
            "wireless headset",
            "bt "
        ],
        &[("bluetooth", "Bluetooth"), ("audio", "Audio")]
    );
    add_if!(
        &[
            "camera",
            "webcam",
            "video call",
            "camera not working",
            "can't see camera"
        ],
        &[("camera", "Camera")]
    );
    add_if!(
        &["teams", "microsoft teams"],
        &[
            ("teams", "Teams"),
            ("identity_auth", "Identity & Auth"),
            ("browser_health", "Browser Health")
        ]
    );
    add_if!(
        &["outlook", "email not working", "mail not", "calendar not"],
        &[("outlook", "Outlook"), ("identity_auth", "Identity & Auth")]
    );
    add_if!(
        &[
            "browser",
            "chrome",
            "edge ",
            "firefox",
            "slow browser",
            "browser crash",
            "browser not"
        ],
        &[("browser_health", "Browser Health")]
    );
    add_if!(
        &[
            "sign in",
            "can't log in",
            "login fail",
            "password",
            "pin not working",
            "fingerprint",
            "hello not",
            "locked out",
            "authentication fail"
        ],
        &[
            ("sign_in", "Sign-In / Windows Hello"),
            ("identity_auth", "Identity & Auth"),
            ("credentials", "Credentials")
        ]
    );
    add_if!(
        &[
            "rdp",
            "remote desktop",
            "can't connect remotely",
            "remote desktop not"
        ],
        &[
            ("rdp", "Remote Desktop"),
            ("connectivity", "Connectivity"),
            ("firewall_rules", "Firewall Rules")
        ]
    );
    add_if!(
        &[
            "device not recognized",
            "driver not",
            "usb not working",
            "device problem",
            "yellow bang",
            "hardware not"
        ],
        &[
            ("device_health", "Device Health"),
            ("drivers", "Drivers"),
            ("peripherals", "Peripherals")
        ]
    );
    add_if!(
        &[
            "time wrong",
            "clock wrong",
            "wrong time",
            "time sync",
            "time off"
        ],
        &[("ntp", "NTP / Time Sync")]
    );
    add_if!(
        &[
            "onedrive",
            "one drive",
            "file sync",
            "not syncing",
            "sync fail"
        ],
        &[("onedrive", "OneDrive")]
    );
    add_if!(
        &["wmi error", "powershell wmi", "get-wmiobject fail"],
        &[("wmi_health", "WMI Health")]
    );
    add_if!(
        &[
            "monitor",
            "display",
            "screen resolution",
            "second monitor",
            "wrong resolution",
            "display settings",
            "refresh rate",
            "scaling"
        ],
        &[("display_config", "Display Config")]
    );
    add_if!(
        &[
            "keyboard not",
            "keyboard stop",
            "keyboard broke",
            "mouse not",
            "mouse stop",
            "mouse broke",
            "touchpad",
            "trackpad",
            "peripheral not"
        ],
        &[
            ("peripherals", "Peripherals"),
            ("device_health", "Device Health")
        ]
    );
    add_if!(
        &[
            "hibernate",
            "won't hibernate",
            "sleep issue",
            "won't sleep",
            "won't wake",
            "stuck after sleep",
            "won't wake up",
            "sleep mode"
        ],
        &[
            ("pending_reboot", "Pending Reboot"),
            ("services", "Services"),
            ("thermal", "Thermal")
        ]
    );
    add_if!(
        &[
            "microsoft store",
            "store app",
            "windows store",
            "uwp",
            "app won't install",
            "store not working",
            "winget"
        ],
        &[("installer_health", "Installer Health")]
    );
    add_if!(
        &[
            "no sound",
            "audio not",
            "sound not",
            "speaker not",
            "microphone not",
            "mic not",
            "audio stopped",
            "crackling",
            "no audio"
        ],
        &[("audio", "Audio")]
    );
    add_if!(
        &[
            "bluetooth not",
            "bluetooth won't",
            "headset won't connect",
            "headphones won't",
            "can't pair",
            "won't pair",
            "bluetooth disconnect",
            "bluetooth keep"
        ],
        &[("bluetooth", "Bluetooth")]
    );
    add_if!(
        &[
            "outlook not",
            "outlook won't",
            "outlook crash",
            "outlook slow",
            "email not",
            "email won't",
            "email crash",
            "calendar not",
            "pst",
            "ost file"
        ],
        &[("outlook", "Outlook")]
    );
    add_if!(
        &[
            "teams not",
            "teams won't",
            "teams crash",
            "teams slow",
            "teams black screen",
            "teams audio",
            "teams video",
            "microsoft teams"
        ],
        &[("teams", "Teams")]
    );
    add_if!(
        &[
            "chrome slow",
            "chrome crash",
            "edge slow",
            "edge crash",
            "firefox slow",
            "firefox crash",
            "browser slow",
            "browser crash",
            "browser not",
            "browser keeps"
        ],
        &[("browser_health", "Browser Health")]
    );

    add_if!(
        &[
            "screen flicker",
            "display flicker",
            "screen blink",
            "monitor flicker",
            "display artifact",
            "screen glitch",
            "black screen",
            "screen goes black"
        ],
        &[
            ("display_config", "Display Config"),
            ("device_health", "Device Health"),
            ("drivers", "Drivers")
        ]
    );
    add_if!(
        &[
            "high disk",
            "disk 100",
            "disk usage",
            "disk is full",
            "disk almost full",
            "disk at 100",
            "storage full",
            "no space left"
        ],
        &[
            ("storage", "Storage"),
            ("processes", "Processes"),
            ("disk_health", "Disk Health")
        ]
    );
    add_if!(
        &[
            "gpu",
            "graphics",
            "game slow",
            "games not",
            "gaming performance",
            "fps low",
            "fps drop",
            "game crash",
            "video card",
            "graphics card",
            "stuttering"
        ],
        &[
            ("overclocker", "GPU / Overclocker Telemetry"),
            ("thermal", "Thermal"),
            ("resource_load", "Resource Load"),
            ("drivers", "Drivers")
        ]
    );
    add_if!(
        &[
            "nvlddmkm",
            "amdkmdag",
            "tdr failure",
            "video_tdr_failure",
            "gpu driver crash",
            "gpu driver stopped",
            "gpu hang",
            "display adapter error"
        ],
        &[
            ("device_health", "Device Health"),
            ("drivers", "Drivers"),
            ("recent_crashes", "Recent Crashes")
        ]
    );
    add_if!(
        &[
            "startup slow",
            "boot slow",
            "slow boot",
            "takes long to start",
            "slow to start",
            "long boot"
        ],
        &[
            ("startup_items", "Startup Items"),
            ("resource_load", "Resource Load"),
            ("services", "Services")
        ]
    );
    add_if!(
        &[
            "no sound",
            "audio not",
            "sound not",
            "speaker not",
            "microphone not",
            "mic not",
            "audio stopped",
            "crackling audio",
            "distorted sound",
            "no audio"
        ],
        &[("audio", "Audio")]
    );
    add_if!(
        &[
            "install app",
            "install fail",
            "can't install",
            "installation fail",
            "app won't install",
            "setup fail",
            "winget fail",
            "store install"
        ],
        &[
            ("installer_health", "Installer Health"),
            ("pending_reboot", "Pending Reboot")
        ]
    );
    add_if!(
        &[
            "usb",
            "usb not recognized",
            "usb not working",
            "device not recognized",
            "device manager error",
            "yellow bang",
            "pnp error",
            "driver missing"
        ],
        &[
            ("device_health", "Device Health"),
            ("drivers", "Drivers"),
            ("usb_history", "USB History")
        ]
    );
    add_if!(
        &[
            "no wifi",
            "no wi-fi",
            "can't find wifi",
            "can't find wi-fi",
            "wifi not showing",
            "no wireless networks",
            "no networks found",
            "wifi adapter",
            "wireless adapter"
        ],
        &[
            ("wifi", "Wi-Fi"),
            ("device_health", "Device Health"),
            ("drivers", "Drivers")
        ]
    );
    add_if!(
        &[
            "network share",
            "shared folder",
            "mapped drive",
            "unc path",
            "can't access share",
            "network drive",
            "smb share",
            "\\\\server",
            "file share"
        ],
        &[
            ("share_access", "Share Access"),
            ("connectivity", "Connectivity"),
            ("shares", "Shares")
        ]
    );
    add_if!(
        &[
            "microsoft store",
            "windows store",
            "appx",
            "store not",
            "store won't",
            "store app",
            "wsreset"
        ],
        &[
            ("installer_health", "Installer Health"),
            ("pending_reboot", "Pending Reboot")
        ]
    );
    add_if!(
        &[
            "sleep",
            "hibernate",
            "won't wake",
            "won't sleep",
            "stuck after sleep",
            "wake from sleep",
            "keeps waking",
            "fast startup",
            "power issue",
            "power problem"
        ],
        &[
            ("log_check", "Event Log"),
            ("startup_items", "Startup Items"),
            ("services", "Services")
        ]
    );
    add_if!(
        &[
            "keyboard",
            "mouse not",
            "touchpad",
            "trackpad",
            "input device",
            "keyboard not",
            "mouse frozen",
            "keyboard frozen"
        ],
        &[
            ("peripherals", "Peripherals"),
            ("device_health", "Device Health")
        ]
    );
    add_if!(
        &[
            "bandwidth",
            "high network",
            "network usage",
            "using all",
            "slow internet",
            "data usage",
            "upload slow",
            "download slow",
            "network slow"
        ],
        &[
            ("network_stats", "Network Stats"),
            ("connections", "Active Connections"),
            ("processes", "Processes")
        ]
    );
    add_if!(
        &[
            "wifi disconnects",
            "wifi drops",
            "wifi keeps dropping",
            "wifi keeps disconnecting",
            "internet keeps cutting",
            "connection drops",
            "wifi intermittent",
            "wifi unstable"
        ],
        &[
            ("wifi", "Wi-Fi"),
            ("network_adapter", "Network Adapter"),
            ("connectivity", "Connectivity")
        ]
    );
    add_if!(
        &[
            "access denied",
            "access is denied",
            "permission denied",
            "you don't have permission",
            "cannot access this folder",
            "file access denied",
            "folder access denied",
            "unauthorized access"
        ],
        &[
            ("user_accounts", "User Accounts"),
            ("shares", "Shares")
        ]
    );

    if topics.is_empty() {
        topics.push(("health_report", "System Health"));
        topics.push(("log_check", "Event Log"));
    }
    topics
}

pub fn fix_plan_topics(issue: &str) -> Vec<(&'static str, &'static str)> {
    topics_for_issue(issue)
}

/// A safe auto-executable fix with optional post-fix verification.
/// `verify_topic`: inspect_host topic to re-run before (pre-check) and after the fix.
/// `verify_gone`: pattern ABSENT in healthy output; present means the problem exists.
/// `include_in_sweep`: eligible for `--fix-all` maintenance sweep.
pub struct AutoFix {
    pub label: &'static str,
    pub cmd: &'static str,
    pub verify_topic: Option<&'static str>,
    pub verify_gone: Option<&'static str>,
    pub include_in_sweep: bool,
}

struct AutoCmdAc {
    ac: aho_corasick::AhoCorasick,
    entries: Vec<AutoFix>,
}

static AUTO_CMD_AC: std::sync::OnceLock<AutoCmdAc> = std::sync::OnceLock::new();

fn auto_cmd_ac() -> &'static AutoCmdAc {
    AUTO_CMD_AC.get_or_init(|| {
        // (trigger, label, cmd, verify_topic, verify_gone, include_in_sweep)
        const SAFE: &[(&str, &str, &str, Option<&str>, Option<&str>, bool)] = &[
            (
                "dns: failed",
                "Flush DNS cache",
                "ipconfig /flushdns",
                Some("connectivity"),
                Some("dns: failed"),
                true,
            ),
            (
                "dns resolution: failed",
                "Flush DNS cache",
                "ipconfig /flushdns",
                Some("connectivity"),
                Some("dns: failed"),
                false, // duplicate label — sweep deduplicates by label
            ),
            (
                "wsearch",
                "Restart Windows Search",
                "powershell -Command \"Restart-Service WSearch -ErrorAction SilentlyContinue\"",
                Some("search_index"),
                Some("stopped"),
                true,
            ),
            (
                "windows search",
                "Restart Windows Search",
                "powershell -Command \"Restart-Service WSearch -ErrorAction SilentlyContinue\"",
                Some("search_index"),
                Some("stopped"),
                false, // duplicate label
            ),
            (
                "spooler",
                "Restart Print Spooler",
                "powershell -Command \"Restart-Service Spooler -Force\"",
                Some("printers"),
                Some("offline"),
                true,
            ),
            (
                "print spooler",
                "Restart Print Spooler",
                "powershell -Command \"Restart-Service Spooler -Force\"",
                Some("printers"),
                Some("offline"),
                false, // duplicate label
            ),
            (
                "ntp source unreachable",
                "Resync system clock",
                "w32tm /resync /force",
                Some("ntp"),
                Some("failed"),
                true,
            ),
            (
                "time sync failed",
                "Resync system clock",
                "w32tm /resync /force",
                Some("ntp"),
                Some("failed"),
                false, // duplicate label
            ),
            (
                "bits",
                "Restart BITS service",
                "powershell -Command \"Restart-Service BITS -Force\"",
                None,
                None,
                false, // no verify — skip sweep
            ),
            (
                "wuauserv",
                "Restart Windows Update service",
                "powershell -Command \"Restart-Service wuauserv -Force\"",
                None,
                None,
                false,
            ),
            (
                "windows update service",
                "Restart Windows Update service",
                "powershell -Command \"Restart-Service wuauserv -Force\"",
                None,
                None,
                false, // duplicate label
            ),
            (
                "audiosrv",
                "Restart Audio service",
                "powershell -Command \"Restart-Service Audiosrv -Force\"",
                Some("audio"),
                Some("not running"),
                true,
            ),
            (
                "windows audio",
                "Restart Audio service",
                "powershell -Command \"Restart-Service Audiosrv -Force\"",
                Some("audio"),
                Some("not running"),
                false, // duplicate label
            ),
            (
                "low disk",
                "Empty Recycle Bin",
                "powershell -Command \"Clear-RecycleBin -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                true, // always safe — include in sweep
            ),
            (
                "free up space",
                "Empty Recycle Bin",
                "powershell -Command \"Clear-RecycleBin -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                false, // duplicate label
            ),
            // Teams cache — classic and new Teams
            (
                "teams cache",
                "Clear Teams cache",
                "powershell -Command \"Get-Process ms-teams,Teams -ErrorAction SilentlyContinue | Stop-Process -Force; Remove-Item \\\"$env:APPDATA\\\\Microsoft\\\\Teams\\\\Cache\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item \\\"$env:LOCALAPPDATA\\\\Packages\\\\MSTeams_8wekyb3d8bbwe\\\\LocalCache\\\\Microsoft\\\\MSTeams\\\\EBWebView\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                Some("teams"),
                Some("cache:"),
                true,
            ),
            (
                "msteams",
                "Clear Teams cache",
                "powershell -Command \"Get-Process ms-teams,Teams -ErrorAction SilentlyContinue | Stop-Process -Force; Remove-Item \\\"$env:APPDATA\\\\Microsoft\\\\Teams\\\\Cache\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue; Remove-Item \\\"$env:LOCALAPPDATA\\\\Packages\\\\MSTeams_8wekyb3d8bbwe\\\\LocalCache\\\\Microsoft\\\\MSTeams\\\\EBWebView\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                Some("teams"),
                Some("cache:"),
                false, // duplicate label
            ),
            // Bluetooth
            (
                "bluetooth service",
                "Restart Bluetooth service",
                "powershell -Command \"Restart-Service bthserv -Force -ErrorAction SilentlyContinue\"",
                Some("bluetooth"),
                Some("not running"),
                true,
            ),
            (
                "bthserv",
                "Restart Bluetooth service",
                "powershell -Command \"Restart-Service bthserv -Force -ErrorAction SilentlyContinue\"",
                Some("bluetooth"),
                Some("not running"),
                false, // duplicate label
            ),
            // DHCP — renewing drops network briefly; skip sweep
            (
                "dhcp lease expired",
                "Renew DHCP lease",
                "ipconfig /release && ipconfig /renew",
                Some("dhcp"),
                Some("expired"),
                false,
            ),
            (
                "lease expires",
                "Renew DHCP lease",
                "ipconfig /release && ipconfig /renew",
                Some("dhcp"),
                Some("expired"),
                false, // duplicate label
            ),
            // DNS client service — covered by DNS flush in sweep
            (
                "dnscache",
                "Restart DNS Client service",
                "powershell -Command \"Restart-Service Dnscache -Force -ErrorAction SilentlyContinue\"",
                Some("connectivity"),
                Some("dns: failed"),
                false,
            ),
            // OneDrive
            (
                "onedrive not running",
                "Start OneDrive",
                "powershell -Command \"Start-Process \\\"$env:LOCALAPPDATA\\\\Microsoft\\\\OneDrive\\\\OneDrive.exe\\\" -ErrorAction SilentlyContinue\"",
                Some("onedrive"),
                Some("not running"),
                true,
            ),
            // WMI — disruptive; only on explicit --fix, not sweep
            (
                "wmi repository",
                "Restart WMI service",
                "powershell -Command \"Restart-Service winmgmt -Force\"",
                Some("wmi_health"),
                Some("corrupt"),
                false,
            ),
            (
                "winmgmt",
                "Restart WMI service",
                "powershell -Command \"Restart-Service winmgmt -Force\"",
                Some("wmi_health"),
                Some("corrupt"),
                false, // duplicate label
            ),
            // Network Location Awareness
            (
                "unidentified network",
                "Restart Network Location Awareness",
                "powershell -Command \"Restart-Service NlaSvc -Force -ErrorAction SilentlyContinue\"",
                Some("network_profile"),
                Some("unidentified"),
                true,
            ),
            // Windows Temp folder — always safe, include in sweep
            (
                "temp folder",
                "Clear Windows Temp folder",
                "powershell -Command \"Remove-Item \\\"$env:TEMP\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                true, // always safe — runs every sweep
            ),
            (
                "temporary files",
                "Clear Windows Temp folder",
                "powershell -Command \"Remove-Item \\\"$env:TEMP\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                false, // duplicate label
            ),
            // Windows Firewall
            (
                "firewall: off",
                "Restart Windows Firewall",
                "powershell -Command \"Restart-Service MpsSvc -Force -ErrorAction SilentlyContinue\"",
                Some("security"),
                Some("firewall: off"),
                true,
            ),
            (
                "firewall profile: disabled",
                "Restart Windows Firewall",
                "powershell -Command \"Restart-Service MpsSvc -Force -ErrorAction SilentlyContinue\"",
                Some("security"),
                Some("firewall: off"),
                false, // duplicate label
            ),
            // TCP/IP stack reset — requires reboot; explicit --fix only
            (
                "winsock",
                "Reset TCP/IP stack",
                "netsh int ip reset && netsh winsock reset",
                Some("connectivity"),
                Some("unreachable"),
                false,
            ),
            // Remote Desktop service — explicit --fix only (disruptive if active sessions)
            (
                "termservice",
                "Restart Remote Desktop Services",
                "powershell -Command \"Restart-Service TermService -Force -ErrorAction SilentlyContinue\"",
                Some("rdp"),
                Some("stopped"),
                false,
            ),
            // WLAN AutoConfig — explicit --fix only (could drop Wi-Fi briefly)
            (
                "wlansvc",
                "Restart WLAN AutoConfig service",
                "powershell -Command \"Restart-Service Wlansvc -Force -ErrorAction SilentlyContinue\"",
                Some("wifi"),
                Some("stopped"),
                false,
            ),
            // Cryptographic Services
            (
                "cryptsvc",
                "Restart Cryptographic Services",
                "powershell -Command \"Restart-Service CryptSvc -Force -ErrorAction SilentlyContinue\"",
                Some("identity_auth"),
                Some("cryptsvc"),
                false,
            ),
            // Microsoft Store — wsreset is safe, clears cache only
            (
                "microsoft.windowsstore | status: missing",
                "Reset Microsoft Store cache",
                "wsreset.exe",
                None,
                None,
                false, // interactive window opens; not suitable for sweep
            ),
            // Browser cache — Edge only (Chrome requires profile path resolution; explicit --fix only)
            (
                "browser slow",
                "Clear Microsoft Edge cache",
                "powershell -Command \"Remove-Item \\\"$env:LOCALAPPDATA\\\\Microsoft\\\\Edge\\\\User Data\\\\Default\\\\Cache\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                false, // user data — explicit --fix only, never sweep
            ),
            (
                "browser crashing",
                "Clear Microsoft Edge cache",
                "powershell -Command \"Remove-Item \\\"$env:LOCALAPPDATA\\\\Microsoft\\\\Edge\\\\User Data\\\\Default\\\\Cache\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue\"",
                None,
                None,
                false, // duplicate label
            ),
            // VPN — RasMan manages all VPN tunnels; safe to restart
            (
                "rasman",
                "Restart Remote Access Connection Manager (VPN)",
                "powershell -Command \"Restart-Service RasMan -Force -ErrorAction SilentlyContinue\"",
                Some("vpn"),
                Some("not running"),
                false, // could drop active VPN tunnel; explicit --fix only
            ),
            (
                "vpn adapter detected",
                "Restart Remote Access Connection Manager (VPN)",
                "powershell -Command \"Restart-Service RasMan -Force -ErrorAction SilentlyContinue\"",
                Some("vpn"),
                Some("not running"),
                false, // duplicate label
            ),
            // Windows Update cache reset — stops services, deletes SoftwareDistribution; explicit only
            (
                "update stuck downloading",
                "Reset Windows Update components",
                "powershell -Command \"net stop wuauserv; net stop bits; Remove-Item \\\"C:\\\\Windows\\\\SoftwareDistribution\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue; net start wuauserv; net start bits\"",
                None,
                None,
                false, // disruptive — never include in sweep
            ),
            (
                "update error 0x",
                "Reset Windows Update components",
                "powershell -Command \"net stop wuauserv; net stop bits; Remove-Item \\\"C:\\\\Windows\\\\SoftwareDistribution\\\\*\\\" -Recurse -Force -ErrorAction SilentlyContinue; net start wuauserv; net start bits\"",
                None,
                None,
                false, // duplicate label
            ),
            // Remote Desktop — security-sensitive; only on explicit --fix
            (
                "remote desktop disabled",
                "Enable Remote Desktop",
                "powershell -Command \"Set-ItemProperty -Path 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' -Name fDenyTSConnections -Value 0; Enable-NetFirewallRule -DisplayGroup 'Remote Desktop' -ErrorAction SilentlyContinue\"",
                Some("rdp"),
                Some("disabled"),
                false,
            ),
        ];
        let mut patterns: Vec<&str> = Vec::with_capacity(SAFE.len());
        let mut entries: Vec<AutoFix> = Vec::with_capacity(SAFE.len());
        for &(trigger, label, cmd, verify_topic, verify_gone, include_in_sweep) in SAFE {
            patterns.push(trigger);
            entries.push(AutoFix { label, cmd, verify_topic, verify_gone, include_in_sweep });
        }
        AutoCmdAc {
            ac: aho_corasick::AhoCorasick::new(&patterns).expect("valid patterns"),
            entries,
        }
    })
}

pub fn fix_plan_auto_commands(combined_output: &str) -> Vec<&'static AutoFix> {
    let lower = combined_output.to_ascii_lowercase();
    let state = auto_cmd_ac();
    let mut seen_labels = std::collections::HashSet::new();
    let mut result: Vec<&'static AutoFix> = Vec::new();
    for mat in state.ac.find_iter(&lower) {
        let fix = &state.entries[mat.pattern().as_usize()];
        if seen_labels.insert(fix.label) {
            result.push(fix);
        }
    }
    result
}

/// Returns the ordered list of fixes eligible for `--fix-all` maintenance sweep.
/// One entry per label, sweep-flagged only.
pub fn sweep_auto_fixes() -> Vec<&'static AutoFix> {
    let state = auto_cmd_ac();
    let mut seen = std::collections::HashSet::new();
    state
        .entries
        .iter()
        .filter(|f| f.include_in_sweep && seen.insert(f.label))
        .collect()
}

/// Map a recipe title to the most natural `hematite --fix "<issue>"` argument.
/// Returns `None` for recipes that don't have a clean fix-command equivalent.
fn recipe_title_to_fix_arg(title: &str) -> Option<&'static str> {
    match title {
        t if t.contains("disk space") || t.contains("Low disk") => Some("disk full"),
        t if t.contains("Drive health") || t.contains("failure") => Some("disk health warning"),
        t if t.contains("Restart required") => Some("restart required"),
        t if t.contains("event log errors") => Some("Windows errors in event log"),
        t if t.contains("service not running") => Some("critical service stopped"),
        t if t.contains("No internet") => Some("can't connect to internet"),
        t if t.contains("latency") => Some("high network latency"),
        t if t.contains("memory usage") => Some("high RAM usage"),
        t if t.contains("running hot") || t.contains("CPU") => Some("CPU running hot"),
        t if t.contains("security protection") => Some("Windows Defender disabled"),
        t if t.contains("Threat detected") => Some("virus or malware detected"),
        t if t.contains("updates pending") => Some("Windows updates pending"),
        t if t.contains("Hardware device error") => Some("hardware device error"),
        t if t.contains("No backup") => Some("no backup configured"),
        t if t.contains("SMB1") => Some("SMB1 security risk"),
        t if t.contains("encryption not enabled") => Some("BitLocker not enabled"),
        t if t.contains("DNS resolution") => Some("DNS not resolving"),
        t if t.contains("Application crashing") => Some("app crashing repeatedly"),
        t if t.contains("Wi-Fi signal weak") => Some("weak Wi-Fi signal"),
        t if t.contains("clock not synchronizing") => Some("clock out of sync"),
        t if t.contains("system file corruption") => Some("Windows system files corrupt"),
        t if t.contains("Service failed") => Some("service stopped unexpectedly"),
        t if t.contains("Remote Desktop") => Some("RDP disabled or blocked"),
        t if t.contains("Windows Update service") => Some("Windows Update broken"),
        t if t.contains("Teams cache") => Some("Teams not working"),
        t if t.contains("authentication broker") => Some("Microsoft 365 sign-in broken"),
        t if t.contains("WMI repository") => Some("WMI errors"),
        t if t.contains("Windows not activated") => Some("Windows not activated"),
        t if t.contains("Windows Search") => Some("Windows search not finding files"),
        t if t.contains("OneDrive not syncing") => Some("OneDrive not syncing"),
        t if t.contains("Printer offline") => Some("printer offline or stuck"),
        t if t.contains("Outlook mail profile") => Some("Outlook not opening"),
        t if t.contains("PrintNightmare") => Some("PrintNightmare not mitigated"),
        t if t.contains("Temp folder") => Some("disk full"),
        t if t.contains("Windows Firewall") => Some("Windows Firewall stopped"),
        t if t.contains("TCP/IP stack") => Some("network not working after update"),
        t if t.contains("Remote Desktop Services") => Some("RDP not responding"),
        t if t.contains("WLAN AutoConfig") => Some("WiFi service stopped"),
        t if t.contains("Cryptographic Services") => Some("certificates not loading"),
        t if t.contains("No audio") => Some("no sound"),
        t if t.contains("Bluetooth not working") => Some("Bluetooth not connecting"),
        t if t.contains("App installation failing") => Some("app install not working"),
        t if t.contains("Blue screen") || t.contains("BSOD") => Some("BSOD or blue screen"),
        t if t.contains("Camera") && t.contains("webcam") => Some("camera not working"),
        t if t.contains("VPN not connecting") => Some("VPN not connecting"),
        t if t.contains("Screen flickering") => Some("screen flickering"),
        t if t.contains("Microphone not working") => Some("microphone not working"),
        t if t.contains("Login") && t.contains("PIN") => Some("PIN not working"),
        t if t.contains("Disk at 100%") => Some("disk at 100%"),
        t if t.contains("USB device not recognized") => Some("USB device not working"),
        t if t.contains("No Wi-Fi networks visible") => Some("no Wi-Fi networks showing"),
        t if t.contains("Network share") && t.contains("accessible") => {
            Some("network share not accessible")
        }
        t if t.contains("Microsoft Store") && t.contains("AppX") => {
            Some("Microsoft Store not working")
        }
        t if t.contains("sleep") && t.contains("hibernate") => Some("PC won't sleep or wake"),
        t if t.contains("Keyboard") && t.contains("mouse") => Some("keyboard not working"),
        t if t.contains("High network usage") => Some("high network usage"),
        t if t.contains("crackling") || t.contains("distortion") || t.contains("stuttering") => {
            Some("audio crackling")
        }
        t if t.contains("Browser") && (t.contains("slow") || t.contains("crashing")) => {
            Some("browser slow or crashing")
        }
        t if t.contains("startup is slow") || t.contains("long time to boot") => {
            Some("slow startup")
        }
        t if t.contains("Update stuck") || t.contains("Update failing") || t.contains("error code") => {
            Some("Windows Update stuck")
        }
        t if t.contains("GPU") && t.contains("driver crash") => Some("GPU driver crash"),
        t if t.contains("Access denied") || t.contains("permission error") => {
            Some("access denied to file or folder")
        }
        t if t.contains("Wi-Fi keeps disconnecting") || t.contains("dropping intermittently") => {
            Some("Wi-Fi keeps dropping")
        }
        _ => None,
    }
}

/// Given the combined plain-text content of a triage or diagnose report, return
/// a deduplicated list of `hematite --fix "<issue>"` suggestions for the IT tech.
/// Only ACTION and INVESTIGATE severity recipes are surfaced.
pub fn suggest_fix_commands(content: &str) -> Vec<String> {
    let recipes = crate::agent::fix_recipes::match_recipes(content);
    let mut seen = std::collections::HashSet::new();
    let mut suggestions: Vec<String> = Vec::new();
    for recipe in recipes {
        if recipe.severity == "MONITOR" {
            continue;
        }
        if let Some(arg) = recipe_title_to_fix_arg(recipe.title) {
            if seen.insert(arg) {
                suggestions.push(format!("  hematite --fix \"{}\"", arg));
            }
        }
    }
    suggestions
}

/// Re-score health from a saved report's plain-text content for terminal display.
/// Wraps the whole content as a single pseudo-section and runs the recipe matcher.
pub fn score_health_from_content(content: &str) -> crate::agent::fix_recipes::HealthScore {
    crate::agent::fix_recipes::score_health(&[("report", content)])
}

pub fn report_has_issues_in_content(content: &str) -> bool {
    for line in content.lines() {
        if line.contains("Health Score:") {
            if let Some(pos) = line.find("Score:") {
                let after = line[pos + 6..]
                    .trim_start()
                    .trim_start_matches('*')
                    .trim_start();
                return !after.starts_with('A');
            }
        }
    }
    false
}

pub fn fix_issue_categories() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "Performance",
            "slow, lag, freeze, hang, high cpu, high ram, unresponsive",
        ),
        (
            "Network",
            "internet, wifi, offline, no connection, can't browse",
        ),
        ("DNS", "dns, name resolution, can't resolve"),
        ("VPN", "vpn, tunnel, remote access"),
        (
            "Disk Space",
            "disk full, out of space, low disk, drive full",
        ),
        (
            "Disk Health",
            "disk fail, smart error, bad sector, drive health",
        ),
        (
            "Slow Boot",
            "slow boot, startup slow, takes forever to boot",
        ),
        (
            "Crash / BSOD",
            "crash, bsod, blue screen, stop error, kernel panic",
        ),
        (
            "App Crashes",
            "app crash, not responding, application error",
        ),
        (
            "Windows Update",
            "update, windows update, patch, stuck on update",
        ),
        (
            "Virus / Malware",
            "virus, malware, hacked, threat, infected, ransomware",
        ),
        ("Firewall", "firewall, blocked port, blocked connection"),
        ("Printer", "printer, printing, print queue, can't print"),
        ("Audio", "sound, audio, no sound, speaker, mic, microphone"),
        ("Bluetooth", "bluetooth, headphones, wireless headset"),
        ("Camera", "camera, webcam, video call"),
        ("Teams", "teams, microsoft teams"),
        (
            "Outlook / Email",
            "outlook, email not working, calendar not",
        ),
        ("Browser", "browser, chrome, edge, firefox, slow browser"),
        (
            "Sign-In / PIN",
            "sign in, can't log in, pin not working, fingerprint, locked out",
        ),
        (
            "Remote Desktop",
            "rdp, remote desktop, can't connect remotely",
        ),
        (
            "Driver / Device",
            "device not recognized, driver not, usb not working, yellow bang",
        ),
        ("Clock / Time", "time wrong, clock wrong, time sync"),
        ("OneDrive", "onedrive, file sync, not syncing"),
        ("WMI", "wmi error, powershell wmi"),
        (
            "Display / Monitor",
            "monitor, display, screen resolution, second monitor, refresh rate, scaling",
        ),
        (
            "Keyboard / Mouse",
            "keyboard not working, mouse not working, touchpad, trackpad",
        ),
        (
            "Sleep / Hibernate",
            "hibernate, won't sleep, won't wake, sleep issue, stuck after sleep",
        ),
        (
            "Microsoft Store / Apps",
            "microsoft store, store app, uwp, app won't install, winget",
        ),
        (
            "Network Share",
            "network share, mapped drive, unc path, smb share, can't access share",
        ),
        (
            "High Network Usage",
            "bandwidth, high network usage, slow internet, what's using network",
        ),
        (
            "USB Device",
            "usb not recognized, usb not working, device not recognized",
        ),
        (
            "GPU Driver Crash",
            "gpu driver crash, tdr failure, nvlddmkm, black screen after driver, display driver stopped",
        ),
        (
            "Windows Update Stuck",
            "update stuck, update error 0x, update won't install, cumulative update failed",
        ),
        (
            "Access Denied",
            "access denied, can't open file, permission denied, you don't have permission",
        ),
        (
            "Wi-Fi Dropping",
            "wifi drops, wifi disconnects, internet cuts out, wifi intermittent, wifi unstable",
        ),
    ]
}

pub async fn generate_report_markdown() -> String {
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut sections: Vec<(&str, String)> = Vec::with_capacity(REPORT_TOPICS.len());

    let total = REPORT_TOPICS.len();
    for (i, (topic, label)) in REPORT_TOPICS.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if *topic == "hardware" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.split_once(':').map(|x| x.1) {
                                let h = val.trim().to_string();
                                if !h.is_empty() {
                                    hostname = h;
                                }
                            }
                        }
                    }
                }
                s
            }
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    let section_refs: Vec<(&str, &str)> = sections.iter().map(|(l, o)| (*l, o.as_str())).collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::with_capacity(action_plan.len() + sections.len() * 512 + 256);
    md.push_str("# Hematite Diagnostic Report\n\n");
    let _ = writeln!(md, "**Generated:** {}  ", timestamp);
    let _ = writeln!(md, "**Host:** {}  ", hostname);
    let _ = writeln!(md, "**Hematite:** v{}  ", version);
    let _ = write!(
        md,
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    );
    let _ = write!(md, "> {}\n\n", score.summary_line());
    md.push_str("---\n\n");

    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");

    for (label, output) in &sections {
        let _ = write!(md, "## {}\n\n", label);
        md.push_str("```\n");
        md.push_str(output.trim_end());
        md.push_str("\n```\n\n");
    }

    md
}

struct DiagnosisData {
    timestamp: String,
    hostname: String,
    health_output: String,
    follow_up_outputs: Vec<(&'static str, String)>,
}

async fn run_diagnosis_phases() -> DiagnosisData {
    let timestamp = now_timestamp_string();
    let hostname = hostname_from_env();

    eprintln!("  → System Health (scanning for issues)...");
    let health_args = json!({"topic": "health_report"});
    let health_output = match crate::tools::host_inspect::inspect_host(&health_args).await {
        Ok(s) => s,
        Err(e) => format!("Error running health_report: {}", e),
    };

    let follow_up_topics = crate::agent::diagnose::triage_follow_up_topics(&health_output);

    if follow_up_topics.is_empty() {
        eprintln!("  → No follow-up checks needed.");
    } else {
        eprintln!(
            "  → {} area(s) flagged — running targeted checks...",
            follow_up_topics.len()
        );
    }

    let mut follow_up_outputs: Vec<(&'static str, String)> =
        Vec::with_capacity(follow_up_topics.len());
    for (i, topic) in follow_up_topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, follow_up_topics.len(), topic);
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error: {}", e),
        };
        follow_up_outputs.push((*topic, output));
    }

    DiagnosisData {
        timestamp,
        hostname,
        health_output,
        follow_up_outputs,
    }
}

/// Run a full staged diagnosis — health_report → triage → targeted follow-ups → fix recipes.
/// No TUI, no model required. Output is self-contained markdown for cloud model ingestion.
pub async fn generate_diagnosis_report() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let data = run_diagnosis_phases().await;

    let mut section_refs: Vec<(&str, &str)> = vec![("health_report", data.health_output.as_str())];
    for (topic, output) in &data.follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md =
        String::with_capacity(action_plan.len() + data.follow_up_outputs.len() * 512 + 256);
    md.push_str("# Hematite Staged Diagnosis Report\n\n");
    let _ = writeln!(md, "**Generated:** {}  ", data.timestamp);
    let _ = writeln!(md, "**Host:** {}  ", data.hostname);
    let _ = writeln!(md, "**Hematite:** v{}  ", version);
    let _ = write!(
        md,
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    );
    let _ = write!(md, "> {}\n\n", score.summary_line());
    md.push_str("---\n\n");
    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    md.push_str("## System Health\n\n```\n");
    md.push_str(data.health_output.trim_end());
    md.push_str("\n```\n\n");

    if !data.follow_up_outputs.is_empty() {
        md.push_str("## Targeted Investigation\n\n");
        for (topic, output) in &data.follow_up_outputs {
            let _ = write!(md, "### {}\n\n```\n", topic);
            md.push_str(output.trim_end());
            md.push_str("\n```\n\n");
        }
    }

    md
}

/// Same as generate_diagnosis_report but outputs a self-contained HTML file.
pub async fn generate_diagnosis_report_html() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let data = run_diagnosis_phases().await;

    let mut section_refs: Vec<(&str, &str)> = vec![("health_report", data.health_output.as_str())];
    for (topic, output) in &data.follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    let mut sections: Vec<(&str, String)> = vec![("System Health", data.health_output.clone())];
    for (topic, output) in &data.follow_up_outputs {
        sections.push((*topic, output.clone()));
    }

    build_html_document(
        "Hematite Staged Diagnosis",
        &data.timestamp,
        &data.hostname,
        version,
        &score,
        &action_plan_html,
        &sections,
    )
}

pub async fn generate_report_json() -> String {
    let timestamp = now_timestamp_string();
    let hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut obj = serde_json::Map::new();
    obj.insert("generated".into(), json!(timestamp));
    obj.insert("host".into(), json!(hostname));
    obj.insert("hematite_version".into(), json!(version));

    let total = REPORT_TOPICS.len();
    for (i, (topic, label)) in REPORT_TOPICS.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = json!({"topic": topic});
        let value = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(output) => json!({"label": label, "output": output}),
            Err(e) => json!({"label": label, "error": e}),
        };
        obj.insert(topic.to_string(), value);
    }

    serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Runs diagnostic topics, writes to `.hematite/reports/health-<timestamp>.md`,
/// and returns `(markdown_content, saved_path)`.
pub async fn save_report_markdown() -> (String, PathBuf) {
    let md = generate_report_markdown().await;
    let path = report_path("md");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

/// Same as `save_report_markdown` but JSON format.
pub async fn save_report_json() -> (String, PathBuf) {
    let json = generate_report_json().await;
    let path = report_path("json");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &json);
    (json, path)
}

/// Self-contained HTML diagnostic report — double-clickable, no external deps.
pub async fn generate_report_html() -> String {
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut sections: Vec<(&str, String)> = Vec::with_capacity(REPORT_TOPICS.len());

    let total = REPORT_TOPICS.len();
    for (i, (topic, label)) in REPORT_TOPICS.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if *topic == "hardware" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.split_once(':').map(|x| x.1) {
                                let h = val.trim().to_string();
                                if !h.is_empty() {
                                    hostname = h;
                                }
                            }
                        }
                    }
                }
                s
            }
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    let section_refs: Vec<(&str, &str)> = sections.iter().map(|(l, o)| (*l, o.as_str())).collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    build_html_document(
        "Hematite Diagnostic Report",
        &timestamp,
        &hostname,
        version,
        &score,
        &action_plan_html,
        &sections,
    )
}

pub async fn save_report_html() -> (String, PathBuf) {
    let html = generate_report_html().await;
    let path = report_path("html");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

pub async fn save_diagnosis_report() -> (String, PathBuf) {
    let md = generate_diagnosis_report().await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("diagnosis-{}.md", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

pub async fn save_diagnosis_report_html() -> (String, PathBuf) {
    let html = generate_diagnosis_report_html().await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("diagnosis-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

pub async fn generate_diagnosis_report_json() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let data = run_diagnosis_phases().await;
    let mut section_refs: Vec<(&str, &str)> = vec![("health_report", data.health_output.as_str())];
    for (topic, output) in &data.follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let score = crate::agent::fix_recipes::score_health(&section_refs);

    // Collect action/investigate titles by scanning each section independently
    let mut seen_titles = std::collections::HashSet::new();
    let mut action_items: Vec<&str> = Vec::new();
    let mut investigate_items: Vec<&str> = Vec::new();
    for (_label, output) in &section_refs {
        for recipe in crate::agent::fix_recipes::match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                match recipe.severity {
                    "ACTION" => action_items.push(recipe.title),
                    "INVESTIGATE" => investigate_items.push(recipe.title),
                    _ => {}
                }
            }
        }
    }

    let mut sections_obj = serde_json::Map::new();
    sections_obj.insert("health_report".into(), json!(data.health_output));
    for (topic, output) in &data.follow_up_outputs {
        sections_obj.insert(topic.to_string(), json!(output));
    }

    let obj = json!({
        "generated": data.timestamp,
        "host": data.hostname,
        "hematite_version": version,
        "grade": score.grade.to_string(),
        "label": score.label,
        "action_count": score.action_count,
        "investigate_count": score.investigate_count,
        "monitor_count": score.monitor_count,
        "action_items": action_items,
        "investigate_items": investigate_items,
        "follow_up_topics": data.follow_up_outputs.iter().map(|(t, _)| *t).collect::<Vec<_>>(),
        "sections": serde_json::Value::Object(sections_obj),
    });
    serde_json::to_string_pretty(&obj).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

pub async fn save_diagnosis_report_json() -> (String, PathBuf) {
    let json = generate_diagnosis_report_json().await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("diagnosis-{}.json", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &json);
    (json, path)
}

fn build_html_document(
    title: &str,
    timestamp: &str,
    hostname: &str,
    version: &str,
    score: &crate::agent::fix_recipes::HealthScore,
    action_plan_html: &str,
    sections: &[(&str, String)],
) -> String {
    use crate::agent::html_template::{build_html_shell, he, COPY_BUTTON_HTML};

    let mut sections_html =
        String::with_capacity(sections.iter().map(|(_, o)| o.len() + 64).sum::<usize>());
    for (label, output) in sections {
        let _ = writeln!(
            sections_html,
            "<details><summary>{}</summary><pre>{}</pre></details>",
            he(label),
            he(output.trim_end())
        );
    }

    let content = format!(
        r#"<header>
<h1>{title}</h1>
<div class="meta">
  <span>Generated: {timestamp}</span>
  <span>Host: {hostname}</span>
  <span>Hematite v{version}</span>
</div>
<div class="score-row">
  <div class="grade g{grade}">{grade}</div>
  <div class="score-info">
    <h2>Health Score: {grade} — {label}</h2>
    <p>{summary}</p>
  </div>
</div>
<p class="grade-intro">{intro}</p>
{copy_btn}
</header>
<section>
<h2>Action Plan</h2>
{action_plan_html}
</section>
<section>
<h2>Diagnostic Data</h2>
{sections_html}
</section>"#,
        title = he(title),
        hostname = he(hostname),
        timestamp = he(timestamp),
        version = he(version),
        grade = score.grade,
        label = he(score.label),
        summary = he(&score.summary_line()),
        intro = he(score.grade_intro()),
        copy_btn = COPY_BUTTON_HTML,
        action_plan_html = action_plan_html,
        sections_html = sections_html,
    );

    let page_title = format!("{} — {}", he(title), he(hostname));
    build_html_shell(&page_title, version, &content)
}

// ── Triage report (IT-first-look, no model required) ─────────────────────────

struct TriageData {
    timestamp: String,
    hostname: String,
    sections: Vec<(&'static str, String)>,
}

async fn run_triage_phases(preset: &str) -> TriageData {
    let topics = triage_topics_for_preset(preset);
    let total = topics.len();
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let mut sections: Vec<(&'static str, String)> = Vec::with_capacity(total);

    for (i, &(topic, label)) in topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = serde_json::json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if topic == "health_report" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.split_once(':').map(|x| x.1) {
                                let h = val.trim().to_string();
                                if !h.is_empty() {
                                    hostname = h;
                                }
                            }
                        }
                    }
                }
                s
            }
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    TriageData {
        timestamp,
        hostname,
        sections,
    }
}

pub async fn generate_triage_report_markdown(preset: &str) -> String {
    let title = triage_preset_title(preset);
    let data = run_triage_phases(preset).await;
    let version = env!("CARGO_PKG_VERSION");

    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::with_capacity(action_plan.len() + data.sections.len() * 512 + 256);
    let _ = write!(md, "# {}\n\n", title);
    let _ = writeln!(md, "**Generated:** {}  ", data.timestamp);
    let _ = writeln!(md, "**Host:** {}  ", data.hostname);
    let _ = writeln!(md, "**Hematite:** v{}  ", version);
    let _ = write!(
        md,
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    );
    let _ = write!(md, "> {}\n\n", score.summary_line());
    md.push_str("---\n\n## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    for (label, output) in &data.sections {
        let _ = write!(md, "## {}\n\n```\n", label);
        md.push_str(output.trim_end());
        md.push_str("\n```\n\n");
    }
    md
}

pub async fn generate_triage_report_html(preset: &str) -> String {
    let title = triage_preset_title(preset);
    let data = run_triage_phases(preset).await;
    let version = env!("CARGO_PKG_VERSION");

    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    build_html_document(
        title,
        &data.timestamp,
        &data.hostname,
        version,
        &score,
        &action_plan_html,
        &data.sections,
    )
}

pub async fn save_triage_report(preset: &str) -> (String, PathBuf) {
    let md = generate_triage_report_markdown(preset).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("triage-{}.md", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

pub async fn save_triage_report_html(preset: &str) -> (String, PathBuf) {
    let html = generate_triage_report_html(preset).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("triage-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

pub async fn generate_triage_report_json(preset: &str) -> String {
    let data = run_triage_phases(preset).await;
    let version = env!("CARGO_PKG_VERSION");
    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);

    let mut seen_titles = std::collections::HashSet::new();
    let mut action_items: Vec<&str> = Vec::new();
    let mut investigate_items: Vec<&str> = Vec::new();
    for (_label, output) in &section_refs {
        for recipe in crate::agent::fix_recipes::match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                match recipe.severity {
                    "ACTION" => action_items.push(recipe.title),
                    "INVESTIGATE" => investigate_items.push(recipe.title),
                    _ => {}
                }
            }
        }
    }

    let mut sections_obj = serde_json::Map::new();
    for (label, output) in &data.sections {
        sections_obj.insert(label.to_string(), json!(output));
    }

    let obj = json!({
        "generated": data.timestamp,
        "host": data.hostname,
        "hematite_version": version,
        "preset": preset,
        "grade": score.grade.to_string(),
        "label": score.label,
        "action_count": score.action_count,
        "investigate_count": score.investigate_count,
        "monitor_count": score.monitor_count,
        "action_items": action_items,
        "investigate_items": investigate_items,
        "sections": serde_json::Value::Object(sections_obj),
    });
    serde_json::to_string_pretty(&obj).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

pub async fn save_triage_report_json(preset: &str) -> (String, PathBuf) {
    let json = generate_triage_report_json(preset).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("triage-{}.json", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &json);
    (json, path)
}

// ── Fix Plan (--fix "<issue>", no model required) ─────────────────────────────

struct FixPlanData {
    timestamp: String,
    hostname: String,
    sections: Vec<(&'static str, String)>,
}

async fn run_fix_plan_phases(issue: &str) -> FixPlanData {
    let initial_topics = topics_for_issue(issue);
    let total = initial_topics.len();
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let mut sections: Vec<(&'static str, String)> = Vec::with_capacity(total);

    eprintln!("hematite --fix: \"{}\" ({} check(s))", issue, total);
    for (i, &(topic, label)) in initial_topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = serde_json::json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if topic == "health_report" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.split_once(':').map(|x| x.1) {
                                let h = val.trim().to_string();
                                if !h.is_empty() {
                                    hostname = h;
                                }
                            }
                        }
                    }
                }
                s
            }
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    let combined: String = {
        let total = sections.iter().map(|(_, o)| o.len()).sum::<usize>() + sections.len();
        let mut s = String::with_capacity(total);
        for (i, (_, o)) in sections.iter().enumerate() {
            if i > 0 {
                s.push('\n');
            }
            s.push_str(o);
        }
        s
    };
    let ran: Vec<&str> = initial_topics.iter().map(|&(t, _)| t).collect();
    let follow_ups = crate::agent::diagnose::fix_follow_up_topics(&combined, &ran);

    if !follow_ups.is_empty() {
        eprintln!(
            "  → {} follow-up check(s) triggered by findings...",
            follow_ups.len()
        );
    }

    for (i, &(topic, label)) in follow_ups.iter().enumerate() {
        eprintln!("  + [{}/{}] {}...", i + 1, follow_ups.len(), label);
        let args = serde_json::json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    FixPlanData {
        timestamp,
        hostname,
        sections,
    }
}

pub async fn generate_fix_plan_markdown(issue: &str) -> String {
    let data = run_fix_plan_phases(issue).await;
    let version = env!("CARGO_PKG_VERSION");

    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::with_capacity(action_plan.len() + data.sections.len() * 512 + 256);
    md.push_str("# Hematite Fix Plan\n\n");
    let _ = writeln!(md, "**Issue:** {}  ", issue);
    let _ = writeln!(md, "**Generated:** {}  ", data.timestamp);
    let _ = writeln!(md, "**Host:** {}  ", data.hostname);
    let _ = writeln!(md, "**Hematite:** v{}  ", version);
    let _ = write!(
        md,
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    );
    let _ = write!(md, "> {}\n\n", score.summary_line());
    md.push_str("---\n\n## Fix Steps\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    for (label, output) in &data.sections {
        let _ = write!(md, "## {}\n\n```\n", label);
        md.push_str(output.trim_end());
        md.push_str("\n```\n\n");
    }
    md
}

pub async fn generate_fix_plan_html(issue: &str) -> String {
    let data = run_fix_plan_phases(issue).await;
    let version = env!("CARGO_PKG_VERSION");

    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    use crate::agent::html_template::{build_html_shell, he, COPY_BUTTON_HTML};

    let mut sections_html = String::with_capacity(data.sections.len() * 512);
    for (label, output) in &data.sections {
        let _ = writeln!(
            sections_html,
            "<details><summary>{}</summary><pre>{}</pre></details>",
            he(label),
            he(output.trim_end())
        );
    }

    let content = format!(
        r#"<header>
<h1>Fix Plan</h1>
<p class="grade-intro" style="margin-bottom:.85rem">Issue: <strong>{issue}</strong></p>
<div class="meta">
  <span>Generated: {timestamp}</span>
  <span>Host: {hostname}</span>
  <span>Hematite v{version}</span>
</div>
<div class="score-row">
  <div class="grade g{grade}">{grade}</div>
  <div class="score-info">
    <h2>Health Score: {grade} — {label}</h2>
    <p>{summary}</p>
  </div>
</div>
{copy_btn}
</header>
<section>
<h2>Fix Steps</h2>
{action_plan_html}
</section>
<section>
<h2>Diagnostic Data</h2>
{sections_html}
</section>"#,
        issue = he(issue),
        hostname = he(&data.hostname),
        timestamp = he(&data.timestamp),
        version = he(version),
        grade = score.grade,
        label = he(score.label),
        summary = he(&score.summary_line()),
        copy_btn = COPY_BUTTON_HTML,
        action_plan_html = action_plan_html,
        sections_html = sections_html,
    );

    let page_title = format!("Fix Plan: {} — {}", he(issue), he(&data.hostname));
    build_html_shell(&page_title, version, &content)
}

pub async fn save_fix_plan(issue: &str) -> (String, PathBuf) {
    let md = generate_fix_plan_markdown(issue).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("fix-{}.md", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

/// Like `save_fix_plan` but also returns the plain-text action plan for immediate
/// terminal printing. Returns `(action_plan_text, full_md, path)`.
pub async fn save_fix_plan_with_summary(issue: &str) -> (String, String, PathBuf) {
    let data = run_fix_plan_phases(issue).await;
    let version = env!("CARGO_PKG_VERSION");

    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::with_capacity(action_plan.len() + data.sections.len() * 512 + 256);
    md.push_str("# Hematite Fix Plan\n\n");
    let _ = writeln!(md, "**Issue:** {}  ", issue);
    let _ = writeln!(md, "**Generated:** {}  ", data.timestamp);
    let _ = writeln!(md, "**Host:** {}  ", data.hostname);
    let _ = writeln!(md, "**Hematite:** v{}  ", version);
    let _ = write!(
        md,
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    );
    let _ = write!(md, "> {}\n\n", score.summary_line());
    md.push_str("---\n\n## Fix Steps\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    for (label, output) in &data.sections {
        let _ = write!(md, "## {}\n\n```\n", label);
        md.push_str(output.trim_end());
        md.push_str("\n```\n\n");
    }

    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("fix-{}.md", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);

    let summary = format!(
        "Health Score: {} — {}\n\n{}",
        score.grade, score.label, action_plan
    );
    (summary, md, path)
}

pub async fn save_fix_plan_html(issue: &str) -> (String, PathBuf) {
    let html = generate_fix_plan_html(issue).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("fix-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

pub async fn save_fix_plan_json(issue: &str) -> (String, PathBuf) {
    let data = run_fix_plan_phases(issue).await;
    let version = env!("CARGO_PKG_VERSION");
    let section_refs: Vec<(&str, &str)> = data
        .sections
        .iter()
        .map(|(l, o)| (*l, o.as_str()))
        .collect();
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut seen_titles = std::collections::HashSet::new();
    let mut action_items: Vec<&str> = Vec::new();
    let mut investigate_items: Vec<&str> = Vec::new();
    for (_label, output) in &section_refs {
        for recipe in crate::agent::fix_recipes::match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                match recipe.severity {
                    "ACTION" => action_items.push(recipe.title),
                    "INVESTIGATE" => investigate_items.push(recipe.title),
                    _ => {}
                }
            }
        }
    }

    let mut sections_obj = serde_json::Map::new();
    for (label, output) in &data.sections {
        sections_obj.insert(label.to_string(), json!(output));
    }

    let obj = json!({
        "generated": data.timestamp,
        "host": data.hostname,
        "hematite_version": version,
        "issue": issue,
        "grade": score.grade.to_string(),
        "label": score.label,
        "action_count": score.action_count,
        "investigate_count": score.investigate_count,
        "action_items": action_items,
        "investigate_items": investigate_items,
        "action_plan": action_plan,
        "sections": serde_json::Value::Object(sections_obj),
    });
    let json_str =
        serde_json::to_string_pretty(&obj).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("fix-{}.json", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &json_str);
    (json_str, path)
}

// ── Direct topic inspection (--inspect, /query) ──────────────────────────────

/// Run one or more inspect_host topics by name and return combined plain-text output.
/// `topics_csv` is a comma-separated list: e.g. "wifi,latency,dns_cache".
pub async fn generate_inspect_output(topics_csv: &str) -> String {
    let topics: Vec<&str> = topics_csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if topics.is_empty() {
        return "No topics specified. Example: hematite --inspect wifi,latency,dns_cache\n\
                Run `hematite --inventory` to list all 128+ available topics.\n"
            .to_string();
    }

    let total = topics.len();
    if total > 1 {
        eprintln!("hematite --inspect: {} topic(s)", total);
    }
    let mut out = String::new();
    for (i, topic) in topics.iter().enumerate() {
        if total > 1 {
            eprintln!("  [{}/{}] {}...", i + 1, total, topic);
        }
        let args = json!({"topic": topic});
        let result = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error ({}): {}", topic, e),
        };
        if total > 1 {
            let _ = writeln!(out, "─── {} ───", topic);
        }
        out.push_str(result.trim_end());
        out.push('\n');
        if total > 1 {
            out.push('\n');
        }
    }
    out
}

/// Run one or more inspect_host topics and return structured JSON output.
pub async fn generate_inspect_output_json(topics_csv: &str) -> String {
    let topics: Vec<&str> = topics_csv
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if topics.is_empty() {
        return json!({"error": "No topics specified."}).to_string();
    }

    let total = topics.len();
    eprintln!("hematite --inspect (json): {} topic(s)", total);
    let mut sections_obj = serde_json::Map::new();
    let mut combined = String::new();
    for (i, topic) in topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, topic);
        let args = json!({"topic": topic});
        let result = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error ({}): {}", topic, e),
        };
        sections_obj.insert(topic.to_string(), json!(result));
        combined.push_str(result.trim_end());
        combined.push('\n');
    }

    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    let obj = json!({
        "generated": now_timestamp_string(),
        "host": host,
        "hematite_version": env!("CARGO_PKG_VERSION"),
        "topics": topics,
        "sections": serde_json::Value::Object(sections_obj),
        "combined_output": combined,
    });
    serde_json::to_string_pretty(&obj).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Run inspect topics and optionally save as a report file.
/// Returns `(content, saved_path_option)`.
pub async fn run_inspect_topics(
    topics_csv: &str,
    fmt: &str,
    save: bool,
) -> (String, Option<PathBuf>) {
    let content = if fmt == "json" {
        generate_inspect_output_json(topics_csv).await
    } else {
        generate_inspect_output(topics_csv).await
    };

    if !save {
        return (content, None);
    }

    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("inspect-{}.{}", now_file_timestamp(), fmt));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &content);
    (content, Some(path))
}

/// Route a natural-language query to the appropriate inspect_host topics and run them.
/// Uses `all_host_inspection_topics()` for multi-topic detection, falls back to
/// `preferred_host_inspection_topic()` for single-topic, then "summary" if nothing matches.
pub async fn generate_query_output(query: &str) -> String {
    use crate::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};

    let detected = all_host_inspection_topics(query);
    let topics: Vec<&str> = if !detected.is_empty() {
        detected
    } else {
        match preferred_host_inspection_topic(query) {
            Some(t) => vec![t],
            None => vec!["summary"],
        }
    };

    let total = topics.len();
    eprintln!("hematite --query: {} topic(s) matched", total);
    let mut out = String::new();
    for (i, topic) in topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, topic);
        let args = json!({"topic": topic});
        let result = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error ({}): {}", topic, e),
        };
        if total > 1 {
            let _ = writeln!(out, "─── {} ───", topic);
        }
        out.push_str(result.trim_end());
        out.push('\n');
        if total > 1 {
            out.push('\n');
        }
    }
    out
}

/// Like `generate_query_output` but returns structured JSON with matched topics + per-topic output.
pub async fn generate_query_output_json(query: &str) -> String {
    use crate::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};

    let detected = all_host_inspection_topics(query);
    let topics: Vec<&str> = if !detected.is_empty() {
        detected
    } else {
        match preferred_host_inspection_topic(query) {
            Some(t) => vec![t],
            None => vec!["summary"],
        }
    };

    let total = topics.len();
    eprintln!("hematite --query (json): {} topic(s) matched", total);
    let mut sections_obj = serde_json::Map::new();
    let mut combined = String::new();
    for (i, topic) in topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, topic);
        let args = json!({"topic": topic});
        let result = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error ({}): {}", topic, e),
        };
        sections_obj.insert(topic.to_string(), json!(result));
        combined.push_str(result.trim_end());
        combined.push('\n');
    }

    let obj = json!({
        "query": query,
        "matched_topics": topics,
        "sections": serde_json::Value::Object(sections_obj),
        "combined_output": combined,
    });
    serde_json::to_string_pretty(&obj).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Save arbitrary markdown content as a dark-theme HTML page.
/// Returns `(html_string, saved_path)`. Title defaults to a timestamp slug
/// if empty. Saves to `.hematite/reports/research-DATE.html`.
pub fn save_research_html(title: &str, body_md: &str) -> (String, PathBuf) {
    use crate::agent::html_template::{build_html_shell, he, markdown_to_html, COPY_BUTTON_HTML};
    let version = env!("CARGO_PKG_VERSION");
    let timestamp = now_timestamp_string();
    let display_title = if title.trim().is_empty() {
        format!("Research — {}", &timestamp[..10])
    } else {
        title.to_string()
    };

    let body_html = markdown_to_html(body_md);
    let content = format!(
        r#"<header>
<h1>{title}</h1>
<div class="meta">
  <span>Saved: {timestamp}</span>
  <span>Hematite v{version}</span>
</div>
{copy_btn}
</header>
<section>
{body_html}
</section>"#,
        title = he(&display_title),
        timestamp = he(&timestamp),
        version = he(version),
        copy_btn = COPY_BUTTON_HTML,
        body_html = body_html,
    );

    let html = build_html_shell(&display_title, version, &content);
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("research-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

fn report_path(ext: &str) -> PathBuf {
    crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("health-{}.{}", now_file_timestamp(), ext))
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

pub fn timestamp_label() -> String {
    now_timestamp_string()
}

fn now_timestamp_string() -> String {
    let now = unix_now();
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(now);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        y, mo, d, h, mi, s
    )
}

fn now_file_timestamp() -> String {
    let now = unix_now();
    let (y, mo, d, h, mi, _s) = epoch_to_ymd_hms(now);
    format!("{:04}-{:02}-{:02}_{:02}-{:02}", y, mo, d, h, mi)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hostname_from_env() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Gregorian calendar decomposition of a Unix timestamp (accurate 1970–2100).
fn epoch_to_ymd_hms(epoch: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s = (epoch % 60) as u32;
    let mi = ((epoch / 60) % 60) as u32;
    let h = ((epoch / 3600) % 24) as u32;
    let days = epoch / 86400;

    let years_400 = days / 146097;
    let rem = days % 146097;
    let years_100 = rem.min(146096) / 36524;
    let rem = rem - years_100 * 36524;
    let years_4 = rem / 1461;
    let rem = rem % 1461;
    let years_1 = rem.min(1460) / 365;
    let rem = rem - years_1 * 365;

    let year = (1970 + years_400 * 400 + years_100 * 100 + years_4 * 4 + years_1) as u32;
    let leap = u32::from(
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)),
    );
    let month_days: [u32; 12] = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut rem = rem as u32;
    let mut month = 1u32;
    for &md in &month_days {
        if rem < md {
            break;
        }
        rem -= md;
        month += 1;
    }
    let day = rem + 1;
    (year, month, day, h, mi, s)
}
