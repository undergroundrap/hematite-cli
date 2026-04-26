use serde_json::json;
use std::path::PathBuf;

const REPORT_TOPICS: &[(&str, &str)] = &[
    ("health_report", "System Health"),
    ("hardware", "Hardware"),
    ("storage", "Storage"),
    ("network", "Network"),
    ("security", "Security"),
    ("toolchains", "Developer Toolchains"),
];

/// IT-first-look triage topics (health, security, connectivity, identity, updates).
const TRIAGE_TOPICS: &[(&str, &str)] = &[
    ("health_report", "System Health"),
    ("security", "Security Posture"),
    ("connectivity", "Connectivity"),
    ("identity_auth", "Identity & Auth (M365/AAD)"),
    ("updates", "Windows Updates"),
];

fn triage_topics_for_preset(preset: &str) -> &'static [(&'static str, &'static str)] {
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

    if topics.is_empty() {
        topics.push(("health_report", "System Health"));
        topics.push(("log_check", "Event Log"));
    }
    topics
}

/// Public alias for --fix --dry-run: returns the inspection topics that would
/// run for a given issue description without executing anything.
pub fn fix_plan_topics(issue: &str) -> Vec<(&'static str, &'static str)> {
    topics_for_issue(issue)
}

/// Safe non-destructive commands that can be auto-executed with --execute.
/// Each entry: (trigger_substring_in_output, display_label, command_to_run).
/// Only single-service restarts and DNS/clock operations — nothing that
/// modifies files, accounts, firewall rules, or requires a reboot.
pub fn fix_plan_auto_commands(combined_output: &str) -> Vec<(&'static str, &'static str)> {
    const SAFE: &[(&str, &str, &str)] = &[
        ("dns: failed", "Flush DNS cache", "ipconfig /flushdns"),
        ("dns resolution: failed", "Flush DNS cache", "ipconfig /flushdns"),
        ("wsearch", "Restart Windows Search", "powershell -Command \"Restart-Service WSearch -ErrorAction SilentlyContinue\""),
        ("windows search", "Restart Windows Search", "powershell -Command \"Restart-Service WSearch -ErrorAction SilentlyContinue\""),
        ("spooler", "Restart Print Spooler", "powershell -Command \"Restart-Service Spooler -Force\""),
        ("print spooler", "Restart Print Spooler", "powershell -Command \"Restart-Service Spooler -Force\""),
        ("ntp source unreachable", "Resync system clock", "w32tm /resync /force"),
        ("time sync failed", "Resync system clock", "w32tm /resync /force"),
        ("bits", "Restart BITS service", "powershell -Command \"Restart-Service BITS -Force\""),
        ("wuauserv", "Restart Windows Update service", "powershell -Command \"Restart-Service wuauserv -Force\""),
        ("windows update service", "Restart Windows Update service", "powershell -Command \"Restart-Service wuauserv -Force\""),
        ("audiosrv", "Restart Audio service", "powershell -Command \"Restart-Service Audiosrv -Force\""),
        ("windows audio", "Restart Audio service", "powershell -Command \"Restart-Service Audiosrv -Force\""),
        ("low disk", "Empty Recycle Bin", "powershell -Command \"Clear-RecycleBin -Force -ErrorAction SilentlyContinue\""),
        ("free up space", "Empty Recycle Bin", "powershell -Command \"Clear-RecycleBin -Force -ErrorAction SilentlyContinue\""),
    ];

    let lower = combined_output.to_ascii_lowercase();
    let mut seen_labels = std::collections::HashSet::new();
    let mut result: Vec<(&'static str, &'static str)> = Vec::new();
    for &(trigger, label, cmd) in SAFE {
        if lower.contains(trigger) && seen_labels.insert(label) {
            result.push((label, cmd));
        }
    }
    result
}

/// Returns true when report content indicates actionable findings (health grade != A).
/// Works for both markdown ("**Health Score:** B") and HTML ("Health Score: B") formats.
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

/// Human-readable category table for `--fix list`.
/// Each entry: (category_label, example_keywords).
pub fn fix_issue_categories() -> &'static [(&'static str, &'static str)] {
    &[
        ("Performance",      "slow, lag, freeze, hang, high cpu, high ram, unresponsive"),
        ("Network",          "internet, wifi, offline, no connection, can't browse"),
        ("DNS",              "dns, name resolution, can't resolve"),
        ("VPN",              "vpn, tunnel, remote access"),
        ("Disk Space",       "disk full, out of space, low disk, drive full"),
        ("Disk Health",      "disk fail, smart error, bad sector, drive health"),
        ("Slow Boot",        "slow boot, startup slow, takes forever to boot"),
        ("Crash / BSOD",     "crash, bsod, blue screen, stop error, kernel panic"),
        ("App Crashes",      "app crash, not responding, application error"),
        ("Windows Update",   "update, windows update, patch, stuck on update"),
        ("Virus / Malware",  "virus, malware, hacked, threat, infected, ransomware"),
        ("Firewall",         "firewall, blocked port, blocked connection"),
        ("Printer",          "printer, printing, print queue, can't print"),
        ("Audio",            "sound, audio, no sound, speaker, mic, microphone"),
        ("Bluetooth",        "bluetooth, headphones, wireless headset"),
        ("Camera",           "camera, webcam, video call"),
        ("Teams",            "teams, microsoft teams"),
        ("Outlook / Email",  "outlook, email not working, calendar not"),
        ("Browser",          "browser, chrome, edge, firefox, slow browser"),
        ("Sign-In / PIN",    "sign in, can't log in, pin not working, fingerprint, locked out"),
        ("Remote Desktop",   "rdp, remote desktop, can't connect remotely"),
        ("Driver / Device",  "device not recognized, driver not, usb not working, yellow bang"),
        ("Clock / Time",     "time wrong, clock wrong, time sync"),
        ("OneDrive",         "onedrive, file sync, not syncing"),
        ("WMI",              "wmi error, powershell wmi"),
    ]
}

pub async fn generate_report_markdown() -> String {
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut sections: Vec<(&str, String)> = Vec::new();

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
                            if let Some(val) = line.splitn(2, ':').nth(1) {
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

    let mut md = String::new();
    md.push_str("# Hematite Diagnostic Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", timestamp));
    md.push_str(&format!("**Host:** {}  \n", hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!(
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    ));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
    md.push_str("---\n\n");

    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");

    for (label, output) in &sections {
        md.push_str(&format!("## {}\n\n", label));
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

    let mut follow_up_outputs: Vec<(&'static str, String)> = Vec::new();
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

    let mut md = String::new();
    md.push_str("# Hematite Staged Diagnosis Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", data.timestamp));
    md.push_str(&format!("**Host:** {}  \n", data.hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!(
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    ));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
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
            md.push_str(&format!("### {}\n\n```\n", topic));
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
    let mut sections: Vec<(&str, String)> = Vec::new();

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
                            if let Some(val) = line.splitn(2, ':').nth(1) {
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

    let mut sections_html = String::new();
    for (label, output) in sections {
        sections_html.push_str(&format!(
            "<details><summary>{}</summary><pre>{}</pre></details>\n",
            he(label),
            he(output.trim_end())
        ));
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
    let mut sections: Vec<(&'static str, String)> = Vec::new();

    for (i, &(topic, label)) in topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = serde_json::json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if topic == "health_report" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.splitn(2, ':').nth(1) {
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

    let mut md = String::new();
    md.push_str(&format!("# {}\n\n", title));
    md.push_str(&format!("**Generated:** {}  \n", data.timestamp));
    md.push_str(&format!("**Host:** {}  \n", data.hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!(
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    ));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
    md.push_str("---\n\n## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    for (label, output) in &data.sections {
        md.push_str(&format!("## {}\n\n```\n", label));
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

// ── Fix Plan (--fix "<issue>", no model required) ─────────────────────────────

struct FixPlanData {
    timestamp: String,
    hostname: String,
    sections: Vec<(&'static str, String)>,
}

/// Two-phase fix plan collection.
/// Phase 1: keyword-match issue → initial topics.
/// Phase 2: read phase-1 output, detect signals, run up to 3 follow-up topics.
async fn run_fix_plan_phases(issue: &str) -> FixPlanData {
    let initial_topics = topics_for_issue(issue);
    let total = initial_topics.len();
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let mut sections: Vec<(&'static str, String)> = Vec::new();

    for (i, &(topic, label)) in initial_topics.iter().enumerate() {
        eprintln!("  [{}/{}] {}...", i + 1, total, label);
        let args = serde_json::json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if topic == "health_report" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.splitn(2, ':').nth(1) {
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

    // Phase 2: self-chain — read what was found and drill deeper
    let combined: String = sections
        .iter()
        .map(|(_, o)| o.as_str())
        .collect::<Vec<_>>()
        .join("\n");
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

    let mut md = String::new();
    md.push_str("# Hematite Fix Plan\n\n");
    md.push_str(&format!("**Issue:** {}  \n", issue));
    md.push_str(&format!("**Generated:** {}  \n", data.timestamp));
    md.push_str(&format!("**Host:** {}  \n", data.hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!(
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    ));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
    md.push_str("---\n\n## Fix Steps\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    for (label, output) in &data.sections {
        md.push_str(&format!("## {}\n\n```\n", label));
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

    let mut sections_html = String::new();
    for (label, output) in &data.sections {
        sections_html.push_str(&format!(
            "<details><summary>{}</summary><pre>{}</pre></details>\n",
            he(label),
            he(output.trim_end())
        ));
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

pub async fn save_fix_plan_html(issue: &str) -> (String, PathBuf) {
    let html = generate_fix_plan_html(issue).await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("fix-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
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

fn ensure_parent(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
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
    let leap = u32::from(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
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
