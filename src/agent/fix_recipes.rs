/// Fix recipe lookup table.
///
/// Maps finding patterns from health_report / inspect_host output to
/// first-line action steps. No model required — this is curated knowledge,
/// not inference. Covers ~80% of what an IT person sees in a normal day.

pub struct Recipe {
    pub severity: &'static str, // "ACTION", "INVESTIGATE", "MONITOR"
    pub title: &'static str,
    pub steps: &'static [&'static str],
    pub dig_deeper: Option<&'static str>, // inspect_host topic to run for more detail
}

/// Match a health_report or inspect_host output line against known recipes.
/// Returns all recipes that apply, in priority order.
pub fn match_recipes(output: &str) -> Vec<&'static Recipe> {
    let lower = output.to_ascii_lowercase();
    let mut matches: Vec<&'static Recipe> = Vec::new();

    for recipe in ALL_RECIPES {
        if recipe.triggers.iter().any(|t| lower.contains(t)) {
            matches.push(&recipe.recipe);
        }
    }

    matches
}

struct RecipeEntry {
    triggers: &'static [&'static str],
    recipe: Recipe,
}

static ALL_RECIPES: &[RecipeEntry] = &[
    // ── Disk / Storage ────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["very low", "disk:", "free space"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Low disk space",
            steps: &[
                "Open Disk Cleanup: press Win+R → type 'cleanmgr' → select C: → check all boxes including 'Windows Update Cleanup'",
                "Empty the Recycle Bin: right-click desktop icon → Empty Recycle Bin",
                "Clear Temp folder: press Win+R → type '%temp%' → Ctrl+A → Delete (skip files in use)",
                "Check largest folders: open PowerShell → Get-ChildItem C:\\ -Recurse -ErrorAction SilentlyContinue | Sort-Object Length -Descending | Select-Object -First 20 FullName, Length",
                "If space is still tight, run: winget install -e --id Microsoft.PowerToys then use PowerToys Disk Space Analyzer",
            ],
            dig_deeper: Some("storage"),
        },
    },
    RecipeEntry {
        triggers: &["disk_health", "smart", "predictive failure", "wear"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Drive health warning — possible failure",
            steps: &[
                "Back up your important files immediately before doing anything else",
                "Verify the SMART status: open PowerShell (admin) → Get-PhysicalDisk | Select FriendlyName, HealthStatus, OperationalStatus",
                "If HealthStatus is 'Unhealthy' or 'Warning', replace the drive — do not wait",
                "For SSDs: check manufacturer's NVMe/SSD tool (Samsung Magician, Crucial Storage Executive, etc.) for wear level",
            ],
            dig_deeper: Some("disk_health"),
        },
    },

    // ── Reboot ────────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["pending reboot", "restart when convenient", "reboot required"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Restart required",
            steps: &[
                "Save your work and restart the computer — pending file operations and updates cannot apply until you do",
                "After restarting, run this report again to confirm the reboot flag cleared",
                "If the flag persists after a restart, check Windows Update: Settings → Windows Update → View update history → look for stuck installs",
            ],
            dig_deeper: Some("pending_reboot"),
        },
    },

    // ── Event log errors ──────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["critical/error event", "error events in windows event log", "critical error"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Windows event log errors detected",
            steps: &[
                "Find the top error sources: PowerShell → Get-WinEvent -FilterHashtable @{LogName='System','Application';Level=1,2} -MaxEvents 100 | Group-Object ProviderName | Sort-Object Count -Descending | Select -First 10",
                "One crashing service or driver usually causes most of the noise — focus on the source with the highest count",
                "For 'Service Control Manager' errors: check which service is crashing → Get-WinEvent -FilterHashtable @{LogName='System';ProviderName='Service Control Manager';Level=2} -MaxEvents 10 | Select Message",
                "For application crashes: check AppEvent for the faulting app name → Get-WinEvent -FilterHashtable @{LogName='Application';Level=2} -MaxEvents 10 | Select TimeCreated,Message",
            ],
            dig_deeper: Some("log_check"),
        },
    },

    // ── Services ──────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["critical service", "not running: windefend", "not running: eventlog", "not running: dnscache"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Critical Windows service not running",
            steps: &[
                "Open Services: press Win+R → type 'services.msc' → Enter",
                "Find the stopped service, right-click → Start",
                "If it fails to start, right-click → Properties → Recovery tab → set 'First failure' to 'Restart the Service'",
                "For Windows Defender (WinDefend) stopped: open Windows Security → Virus & threat protection → turn on Real-time protection",
                "If EventLog is stopped, restart is required — this service cannot be started manually once stopped",
            ],
            dig_deeper: Some("services"),
        },
    },

    // ── Network ───────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["internet connectivity: unreachable", "could not ping 1.1.1.1"],
        recipe: Recipe {
            severity: "ACTION",
            title: "No internet connectivity",
            steps: &[
                "Check physical connection: is the Ethernet cable plugged in, or is Wi-Fi connected?",
                "Test gateway reachability: PowerShell → Test-Connection (Get-NetRoute -DestinationPrefix '0.0.0.0/0').NextHop -Count 1",
                "Flush DNS cache: PowerShell (admin) → Clear-DnsClientCache",
                "Reset TCP/IP stack: PowerShell (admin) → netsh int ip reset; netsh winsock reset → then restart",
                "If on Wi-Fi: forget the network and reconnect, or try 'netsh wlan disconnect' then 'netsh wlan connect name=\"SSID\"'",
            ],
            dig_deeper: Some("connectivity"),
        },
    },
    RecipeEntry {
        triggers: &["high latency", "ms rtt — high latency"],
        recipe: Recipe {
            severity: "MONITOR",
            title: "High network latency detected",
            steps: &[
                "Run a traceroute to find where the delay is: PowerShell → tracert 1.1.1.1",
                "Check for background bandwidth consumers: Task Manager → Performance → Open Resource Monitor → Network tab",
                "If on Wi-Fi, check signal strength and try moving closer to the router or switching to 5GHz",
                "Check your ISP's status page for outages in your area",
            ],
            dig_deeper: Some("latency"),
        },
    },

    // ── RAM ───────────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["ram:", "very low", "running a bit low", "free of"],
        recipe: Recipe {
            severity: "MONITOR",
            title: "High memory usage",
            steps: &[
                "Find the top RAM consumers: Task Manager → Memory column (sort descending)",
                "Close unused browser tabs — each tab can consume 100–500 MB",
                "Check for memory leaks: if one process is growing over time without release, restart it",
                "Disable startup programs that aren't needed: Task Manager → Startup tab → disable high-impact items",
                "If consistently above 85% with normal usage, consider adding RAM",
            ],
            dig_deeper: Some("resource_load"),
        },
    },

    // ── Thermal ───────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["very high", "check cooling", "elevated under load", "°c — very high"],
        recipe: Recipe {
            severity: "ACTION",
            title: "CPU running hot",
            steps: &[
                "Shut down and clean dust from fans and heatsink with compressed air — this is the fix 90% of the time",
                "Check that all fan headers are connected and fans are spinning on boot",
                "Verify thermal paste on CPU heatsink — if it's more than 4 years old and temperatures are high, repaste",
                "In BIOS: confirm fan curve is not set to 'Silent' mode — switch to 'Standard' or 'Performance'",
                "Check for CPU throttling: PowerShell → Get-WmiObject -Class Win32_Processor | Select Name,CurrentClockSpeed,MaxClockSpeed — if Current is much lower than Max under load, it's throttling",
            ],
            dig_deeper: Some("thermal"),
        },
    },

    // ── Security ──────────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["real-time protection: disabled", "defender.*disabled", "firewall.*off"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Windows security protection disabled",
            steps: &[
                "Re-enable Defender real-time protection: Windows Security → Virus & threat protection → turn on Real-time protection",
                "If Defender shows as disabled by a third-party antivirus, ensure that AV is up to date and its own real-time protection is on",
                "Re-enable Windows Firewall: Control Panel → Windows Defender Firewall → Turn Windows Defender Firewall on or off → turn on for all profiles",
                "Run a quick scan: Windows Security → Virus & threat protection → Quick scan",
            ],
            dig_deeper: Some("security"),
        },
    },
    RecipeEntry {
        triggers: &["threat detected", "quarantine", "malware", "virus found"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Threat detected by Windows Defender",
            steps: &[
                "Open Windows Security → Virus & threat protection → Protection history → review detected threats",
                "If action is 'Quarantined', Defender has contained it — review and remove from quarantine",
                "Run a full offline scan: Windows Security → Virus & threat protection → Scan options → Microsoft Defender Offline scan",
                "Change passwords for any accounts accessed on this machine after the infection date",
                "Check browser extensions for anything you didn't install",
            ],
            dig_deeper: Some("defender_quarantine"),
        },
    },

    // ── Windows Update ────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["windows update", "pending update", "update.*required"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Windows updates pending",
            steps: &[
                "Open Settings → Windows Update → Check for updates",
                "Install all available updates, then restart when prompted",
                "If updates are stuck: PowerShell (admin) → net stop wuauserv; net stop bits; net start wuauserv; net start bits",
                "If stuck for more than 24 hours: run the Windows Update Troubleshooter from Settings → System → Troubleshoot → Other troubleshooters",
            ],
            dig_deeper: Some("updates"),
        },
    },
];

pub struct HealthScore {
    pub grade: char,
    pub label: &'static str,
    pub action_count: usize,
    pub investigate_count: usize,
    pub monitor_count: usize,
}

impl HealthScore {
    pub fn summary_line(&self) -> String {
        match (self.action_count, self.investigate_count, self.monitor_count) {
            (0, 0, 0) => "No issues found — machine is healthy.".to_string(),
            (0, 0, m) => format!("{} item(s) to monitor.", m),
            (0, i, 0) => format!("{} item(s) need investigation.", i),
            (0, i, m) => format!("{} item(s) need investigation, {} to monitor.", i, m),
            (a, 0, 0) => format!("{} item(s) require immediate action.", a),
            (a, i, _) => format!(
                "{} item(s) require immediate action, {} need investigation.",
                a, i
            ),
        }
    }
}

/// Compute a health grade (A–F) from diagnostic output sections.
pub fn score_health(outputs: &[(&str, &str)]) -> HealthScore {
    let mut all_recipes: Vec<&Recipe> = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();

    for (_label, output) in outputs {
        for recipe in match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                all_recipes.push(recipe);
            }
        }
    }

    let action_count = all_recipes
        .iter()
        .filter(|r| r.severity == "ACTION")
        .count();
    let investigate_count = all_recipes
        .iter()
        .filter(|r| r.severity == "INVESTIGATE")
        .count();
    let monitor_count = all_recipes
        .iter()
        .filter(|r| r.severity == "MONITOR")
        .count();

    let (grade, label) = if action_count >= 3 {
        ('F', "Critical")
    } else if action_count >= 1 {
        ('D', "Poor")
    } else if investigate_count >= 2 {
        ('C', "Fair")
    } else if investigate_count >= 1 {
        ('B', "Good")
    } else {
        ('A', "Excellent")
    };

    HealthScore {
        grade,
        label,
        action_count,
        investigate_count,
        monitor_count,
    }
}

/// Format all matching recipes for a given diagnostic output into a
/// human-readable action plan section suitable for a Markdown report.
pub fn format_action_plan(outputs: &[(&str, &str)]) -> String {
    let mut all_recipes: Vec<&Recipe> = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();

    for (_label, output) in outputs {
        for recipe in match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                all_recipes.push(recipe);
            }
        }
    }

    if all_recipes.is_empty() {
        return "No actionable findings — machine appears healthy.\n".to_string();
    }

    // Sort: ACTION first, then INVESTIGATE, then MONITOR
    all_recipes.sort_by_key(|r| match r.severity {
        "ACTION" => 0,
        "INVESTIGATE" => 1,
        _ => 2,
    });

    let mut out = String::new();
    for (i, recipe) in all_recipes.iter().enumerate() {
        let badge = match recipe.severity {
            "ACTION" => "⚠ ACTION REQUIRED",
            "INVESTIGATE" => "🔍 INVESTIGATE",
            _ => "📊 MONITOR",
        };
        out.push_str(&format!("### {}. {} — {}\n\n", i + 1, badge, recipe.title));
        for step in recipe.steps {
            out.push_str(&format!("- {}\n", step));
        }
        if let Some(topic) = recipe.dig_deeper {
            out.push_str(&format!(
                "\n*For more detail: run `inspect_host(topic=\"{}\")` or `/diagnose`*\n",
                topic
            ));
        }
        out.push('\n');
    }

    out
}

/// Format all matching recipes as an HTML fragment for embedding in a report page.
pub fn format_action_plan_html(outputs: &[(&str, &str)]) -> String {
    let mut all_recipes: Vec<&Recipe> = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();

    for (_label, output) in outputs {
        for recipe in match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                all_recipes.push(recipe);
            }
        }
    }

    if all_recipes.is_empty() {
        return "<p class=\"healthy\">No actionable findings — machine appears healthy.</p>\n"
            .to_string();
    }

    all_recipes.sort_by_key(|r| match r.severity {
        "ACTION" => 0,
        "INVESTIGATE" => 1,
        _ => 2,
    });

    let mut out = String::new();
    for (i, recipe) in all_recipes.iter().enumerate() {
        let (sev_class, badge_class, badge_text) = match recipe.severity {
            "ACTION" => ("sev-action", "b-action", "ACTION REQUIRED"),
            "INVESTIGATE" => ("sev-investigate", "b-investigate", "INVESTIGATE"),
            _ => ("sev-monitor", "b-monitor", "MONITOR"),
        };
        out.push_str(&format!("<div class=\"recipe {}\">\n", sev_class));
        out.push_str(&format!(
            "<h3><span class=\"badge {}\">{}</span> {}. {}</h3>\n",
            badge_class,
            badge_text,
            i + 1,
            he(recipe.title)
        ));
        out.push_str("<ol>\n");
        for step in recipe.steps {
            out.push_str(&format!("<li>{}</li>\n", he(step)));
        }
        out.push_str("</ol>\n");
        if let Some(topic) = recipe.dig_deeper {
            out.push_str(&format!(
                "<p class=\"dig-deeper\">For more detail: run <code>inspect_host(topic=\"{}\")</code> or <code>/diagnose</code></p>\n",
                he(topic)
            ));
        }
        out.push_str("</div>\n");
    }
    out
}

fn he(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&#34;")
}
