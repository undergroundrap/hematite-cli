/// Symptom-driven cross-topic root-cause diagnosis — no model required.
///
/// Maps a user's plain-English symptom to a curated topic group, runs all
/// topics, collects findings via the fix-recipe engine, and returns a ranked
/// diagnosis with evidence excerpts from the live output.

pub struct SymptomGroup {
    pub category: &'static str,
    pub keywords: &'static [&'static str],
    pub topics: &'static [&'static str],
}

static SYMPTOM_GROUPS: &[SymptomGroup] = &[
    SymptomGroup {
        category: "Performance / Sluggishness",
        keywords: &[
            "slow",
            "sluggish",
            "lag",
            "laggy",
            "freeze",
            "freezing",
            "hang",
            "hanging",
            "unresponsive",
            "high cpu",
            "cpu usage",
            "cpu spike",
            "pc slow",
            "computer slow",
            "running slow",
        ],
        topics: &["resource_load", "processes", "thermal", "pagefile", "disk"],
    },
    SymptomGroup {
        category: "Crash / Blue Screen",
        keywords: &[
            "bsod",
            "blue screen",
            "unexpected restart",
            "restart unexpectedly",
            "kernel panic",
            "stop error",
            "bugcheck",
            "system crash",
            "pc crashed",
            "computer crashed",
            "pc restart unexpectedly",
        ],
        topics: &[
            "recent_crashes",
            "disk_health",
            "drivers",
            "device_health",
            "memory",
        ],
    },
    SymptomGroup {
        category: "No Internet / Offline",
        keywords: &[
            "no internet",
            "no network",
            "can't connect",
            "cannot connect",
            "offline",
            "no connection",
            "internet not working",
            "internet down",
            "can't browse",
            "cannot browse",
            "websites not loading",
        ],
        topics: &["connectivity", "wifi", "dns_cache", "vpn", "proxy"],
    },
    SymptomGroup {
        category: "Slow Internet / Network",
        keywords: &[
            "slow internet",
            "slow network",
            "slow wifi",
            "slow connection",
            "high latency",
            "packet loss",
            "bad ping",
            "buffering",
            "low bandwidth",
            "speed",
            "network slow",
        ],
        topics: &["latency", "wifi", "connections", "network_stats"],
    },
    SymptomGroup {
        category: "Slow Boot / Startup",
        keywords: &[
            "slow startup",
            "slow boot",
            "boot slow",
            "startup slow",
            "takes forever to start",
            "takes long to start",
            "long boot",
            "long startup",
            "slow to start",
            "startup takes",
        ],
        topics: &["startup_items", "services", "disk", "pending_reboot"],
    },
    SymptomGroup {
        category: "Overheating / Thermal",
        keywords: &[
            "hot",
            "overheating",
            "overheat",
            "thermal",
            "fan noise",
            "fan loud",
            "temperature",
            "cpu temp",
            "gpu temp",
            "throttling",
            "running hot",
        ],
        topics: &["thermal", "overclocker", "resource_load"],
    },
    SymptomGroup {
        category: "No Sound / Audio",
        keywords: &[
            "no sound",
            "no audio",
            "audio not working",
            "sound not working",
            "microphone not working",
            "mic not working",
            "speakers not working",
            "headphones not working",
            "audio broken",
            "can't hear",
        ],
        topics: &["audio", "services", "device_health"],
    },
    SymptomGroup {
        category: "Printer / Printing",
        keywords: &[
            "can't print",
            "cannot print",
            "printer not working",
            "print queue",
            "printer offline",
            "stuck print",
            "printing problem",
            "print error",
        ],
        topics: &["printers", "print_spooler", "services"],
    },
    SymptomGroup {
        category: "High Disk Usage",
        keywords: &[
            "disk at 100",
            "disk 100%",
            "disk usage high",
            "high disk usage",
            "disk thrashing",
            "disk maxed",
            "storage full",
            "drive full",
            "low disk space",
            "out of disk space",
        ],
        topics: &[
            "processes",
            "resource_load",
            "storage",
            "storage_deep",
            "pagefile",
            "disk_health",
        ],
    },
    SymptomGroup {
        category: "Windows Updates Failing",
        keywords: &[
            "updates failing",
            "update error",
            "update not working",
            "windows update broken",
            "update stuck",
            "update failed",
            "can't update",
            "cannot update",
            "updates won't install",
        ],
        topics: &["updates", "installer_health", "services", "pending_reboot"],
    },
    SymptomGroup {
        category: "Sign-In / Authentication",
        keywords: &[
            "can't sign in",
            "cannot sign in",
            "login not working",
            "password wrong",
            "pin not working",
            "fingerprint not working",
            "hello not working",
            "can't log in",
            "account locked",
            "authentication failed",
            "sign-in failed",
        ],
        topics: &["sign_in", "identity_auth", "credentials"],
    },
    SymptomGroup {
        category: "Malware / Suspicious Activity",
        keywords: &[
            "virus",
            "malware",
            "infected",
            "suspicious",
            "ransomware",
            "trojan",
            "spyware",
            "adware",
            "defender found",
            "threat",
            "unusual activity",
            "something suspicious",
        ],
        topics: &[
            "defender_quarantine",
            "startup_items",
            "connections",
            "services",
        ],
    },
    SymptomGroup {
        category: "Low Memory / RAM",
        keywords: &[
            "out of memory",
            "low memory",
            "memory leak",
            "ram usage high",
            "not enough memory",
            "memory full",
            "ram maxed",
            "memory pressure",
            "high ram",
            "high memory",
        ],
        topics: &["resource_load", "processes", "pagefile", "storage"],
    },
    SymptomGroup {
        category: "Battery / Power",
        keywords: &[
            "battery",
            "battery dying",
            "battery not charging",
            "short battery",
            "power plan",
            "cpu slow",
            "plugged in not charging",
        ],
        topics: &["battery", "cpu_power", "thermal"],
    },
    SymptomGroup {
        category: "Display / Screen",
        keywords: &[
            "screen flickering",
            "display problem",
            "resolution wrong",
            "wrong resolution",
            "black screen",
            "monitor not detected",
            "display not working",
            "screen issues",
            "flickering",
        ],
        topics: &["display_config", "drivers", "device_health"],
    },
    SymptomGroup {
        category: "Hardware / Device Error",
        keywords: &[
            "device not working",
            "hardware error",
            "hardware not recognized",
            "yellow bang",
            "device manager error",
            "unknown device",
            "peripheral not working",
            "usb not working",
        ],
        topics: &["device_health", "drivers", "peripherals"],
    },
    SymptomGroup {
        category: "Network Share / Drive Access",
        keywords: &[
            "can't access share",
            "network drive not working",
            "mapped drive",
            "shared folder not accessible",
            "smb",
            "unc path",
            "network path",
            "can't see network",
            "network share",
        ],
        topics: &["shares", "services", "connectivity", "firewall_rules"],
    },
    SymptomGroup {
        category: "Microsoft Teams Problems",
        keywords: &[
            "teams not working",
            "teams crashing",
            "teams won't open",
            "teams won't start",
            "teams black screen",
            "teams slow",
            "teams freezing",
            "teams keeps crashing",
            "teams microphone",
            "teams camera",
            "teams audio",
            "teams video",
            "teams sign-in",
            "teams login",
            "teams can't join",
            "teams meeting issue",
            "teams echo",
            "teams no sound",
            "teams not loading",
            "microsoft teams",
            "teams",
            "meeting",
        ],
        topics: &[
            "teams",
            "identity_auth",
            "audio",
            "camera",
            "browser_health",
        ],
    },
    SymptomGroup {
        category: "Outlook / Email Problems",
        keywords: &[
            "outlook not working",
            "outlook crashing",
            "outlook won't open",
            "outlook slow",
            "outlook not syncing",
            "email not working",
            "email not syncing",
            "outlook freezing",
            "outlook keeps crashing",
            "outlook error",
            "can't send email",
            "can't receive email",
            "outlook profile",
            "outlook add-in",
            "new outlook",
            "outlook calendar",
            "ost file",
            "outlook",
            "email not",
        ],
        topics: &["outlook", "identity_auth", "installer_health"],
    },
    SymptomGroup {
        category: "Bluetooth Problems",
        keywords: &[
            "bluetooth not working",
            "bluetooth won't pair",
            "can't pair",
            "bluetooth device not found",
            "bluetooth keeps disconnecting",
            "bluetooth headphones not working",
            "bluetooth speaker not working",
            "bluetooth mouse not working",
            "bluetooth keyboard not working",
            "bluetooth adapter",
            "bluetooth radio",
            "pairing failed",
            "bluetooth audio",
            "airpods not connecting",
        ],
        topics: &["bluetooth", "audio", "device_health", "services"],
    },
    SymptomGroup {
        category: "Camera / Webcam Problems",
        keywords: &[
            "camera not working",
            "webcam not working",
            "camera not detected",
            "camera blocked",
            "camera privacy",
            "webcam black screen",
            "camera in use",
            "camera busy",
            "no camera",
            "webcam not showing",
            "camera not found",
            "video not working",
            "can't use camera",
        ],
        topics: &["camera", "device_health", "drivers", "services"],
    },
    SymptomGroup {
        category: "USB / Device Not Recognized",
        keywords: &[
            "usb not recognized",
            "usb not working",
            "device not recognized",
            "usb device not found",
            "usb keeps disconnecting",
            "usb drops",
            "flash drive not showing",
            "external drive not showing",
            "usb port not working",
            "keyboard not working",
            "mouse not working",
            "external hard drive not detected",
            "usb connection",
        ],
        topics: &["peripherals", "device_health", "drivers", "usb_history"],
    },
    SymptomGroup {
        category: "Sleep / Wake Problems",
        keywords: &[
            "won't sleep",
            "won't wake",
            "can't wake",
            "sleep not working",
            "sleep mode broken",
            "pc won't go to sleep",
            "wakes up randomly",
            "wakes itself",
            "wake on lan",
            "hibernate not working",
            "screen won't turn off",
            "sleep then restart",
            "stuck after sleep",
            "black screen after sleep",
            "won't resume",
        ],
        topics: &[
            "services",
            "device_health",
            "drivers",
            "scheduled_tasks",
            "pending_reboot",
        ],
    },
    SymptomGroup {
        category: "App / Program Crashing",
        keywords: &[
            "app keeps crashing",
            "program crashing",
            "application crash",
            "keeps closing",
            "randomly closes",
            "crashes after opening",
            "word crashing",
            "excel crashing",
            "chrome crashing",
            "edge crashing",
            "firefox crashing",
            "discord crashing",
            "photoshop crashing",
            "app crash",
            "software crash",
            "program won't open",
            "keeps crashing",
        ],
        topics: &[
            "app_crashes",
            "installer_health",
            "device_health",
            "log_check",
        ],
    },
];

/// Find the best matching symptom group for the given user query.
/// Returns `None` if no keywords match.
pub fn match_symptom(query: &str) -> Option<&'static SymptomGroup> {
    let lower = query.to_ascii_lowercase();
    // Pick the group with the most keyword hits for best specificity.
    SYMPTOM_GROUPS
        .iter()
        .map(|g| {
            let hits = g.keywords.iter().filter(|k| lower.contains(*k)).count();
            (hits, g)
        })
        .filter(|(hits, _)| *hits > 0)
        .max_by_key(|(hits, _)| *hits)
        .map(|(_, g)| g)
}

pub struct DiagnosisResult {
    pub category: &'static str,
    pub topics_run: Vec<&'static str>,
    pub findings: Vec<Finding>,
    pub root_causes: Vec<crate::agent::correlation::CorrelatedFinding>,
    pub raw_output: String,
}

pub struct Finding {
    pub severity: &'static str,
    pub title: &'static str,
    pub steps: Vec<&'static str>,
    pub evidence: Vec<String>,
    pub dig_deeper: Option<&'static str>,
}

/// Extract up to `max` lines from `haystack` that contain any of `keywords`.
/// Returns them as evidence excerpts.
fn extract_evidence(haystack: &str, keywords: &[&str], max: usize) -> Vec<String> {
    let lower_hay = haystack.to_ascii_lowercase();
    let mut out: Vec<String> = Vec::new();
    for (line, lower_line) in haystack.lines().zip(lower_hay.lines()) {
        if out.len() >= max {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("──") {
            continue;
        }
        if keywords.iter().any(|k| lower_line.contains(k)) {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Build a diagnosis from raw multi-topic output using the fix-recipe engine.
pub fn build_diagnosis(group: &'static SymptomGroup, raw_output: &str) -> DiagnosisResult {
    use crate::agent::fix_recipes::match_recipes;

    let recipes = match_recipes(raw_output);

    // Sort: ACTION first, INVESTIGATE second, MONITOR last.
    let severity_rank = |s: &str| match s {
        "ACTION" => 0u8,
        "INVESTIGATE" => 1,
        _ => 2,
    };

    let mut sorted_recipes: Vec<_> = recipes.into_iter().collect();
    sorted_recipes.sort_by_key(|r| severity_rank(r.severity));

    let findings = sorted_recipes
        .into_iter()
        .map(|r| {
            // Build evidence: look for lines related to this finding's trigger words
            // by searching with the recipe title keywords in the raw output.
            let title_words: Vec<&str> =
                r.title.split_whitespace().filter(|w| w.len() > 3).collect();
            let evidence = extract_evidence(raw_output, &title_words, 2);
            Finding {
                severity: r.severity,
                title: r.title,
                steps: r.steps.to_vec(),
                evidence,
                dig_deeper: r.dig_deeper,
            }
        })
        .collect();

    let root_causes = crate::agent::correlation::correlate_findings(raw_output);

    DiagnosisResult {
        category: group.category,
        topics_run: group.topics.to_vec(),
        findings,
        root_causes,
        raw_output: raw_output.to_string(),
    }
}
