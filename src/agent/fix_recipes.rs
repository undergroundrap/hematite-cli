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

    // ── Device / driver errors ────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["yellow bang", "pnp error", "configmanager error", "error code 43", "error code 10", "error code 28", "device problem", "driver error"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Hardware device error detected",
            steps: &[
                "Open Device Manager: press Win+R → type 'devmgmt.msc' → Enter",
                "Look for yellow exclamation marks (!) — right-click → Properties → note the error code and device name",
                "Error Code 43 (USB/GPU): unplug and replug the device, or roll back the driver: right-click → Properties → Driver → Roll Back Driver",
                "Error Code 10 (failed to start): update the driver — right-click → Update driver → Search automatically",
                "Error Code 28 (no driver): download the driver from the manufacturer's website (look up the device name + Windows version)",
                "For recurring errors: run SFC scan → PowerShell (admin) → sfc /scannow",
            ],
            dig_deeper: Some("device_health"),
        },
    },

    // ── No backup configured ──────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["file history: disabled", "no backup configured", "no restore points", "last backup: never", "backup: not configured", "file history.*disabled", "no system restore"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "No backup configured",
            steps: &[
                "Enable File History: Settings → System → Storage → Advanced storage settings → Backup options → Add a drive",
                "Enable System Restore: search 'Create a restore point' → select C: → Configure → turn on protection → OK → Create",
                "For a full image backup: search 'Backup and Restore (Windows 7)' → Create a system image → choose an external drive",
                "OneDrive Known Folder Backup covers Desktop/Documents/Pictures: Settings → OneDrive → Backup → Manage backup",
                "Run your first backup immediately — a backup that has never run has zero value",
            ],
            dig_deeper: Some("windows_backup"),
        },
    },

    // ── SMB1 enabled ─────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["smb1 is enabled", "smb1: enabled", "smb1 protocol: enabled", "smb version 1", "smbv1 enabled"],
        recipe: Recipe {
            severity: "ACTION",
            title: "SMB1 protocol enabled — security risk",
            steps: &[
                "SMB1 is a deprecated protocol exploited by WannaCry and NotPetya ransomware — disable it immediately",
                "Disable SMB1: PowerShell (admin) → Set-SmbServerConfiguration -EnableSMB1Protocol $false -Force",
                "Verify it's off: PowerShell → Get-SmbServerConfiguration | Select EnableSMB1Protocol (should show False)",
                "If a legacy device (old NAS, printer) stops working after disabling, upgrade its firmware or replace it — do not re-enable SMB1",
                "Restart required to fully remove the SMB1 listener",
            ],
            dig_deeper: Some("shares"),
        },
    },

    // ── BitLocker not protecting ──────────────────────────────────────────────
    RecipeEntry {
        triggers: &["protection state: off", "bitlocker: off", "bitlocker.*not protecting", "encryption status: fully decrypted", "bitlocker.*disabled"],
        recipe: Recipe {
            severity: "MONITOR",
            title: "Drive encryption not enabled",
            steps: &[
                "BitLocker encrypts your drive so data is unreadable if the laptop is lost or stolen — strongly recommended on portable machines",
                "Enable BitLocker: search 'Manage BitLocker' → Turn on BitLocker for C: → follow the wizard",
                "Save the recovery key to your Microsoft account or print it — you will need it if Windows can't auto-unlock at boot",
                "Encryption runs in the background and takes 1–3 hours for a typical drive — the PC remains usable during this time",
                "Requires TPM 1.2+ or USB key; check: PowerShell → Get-Tpm | Select TpmPresent,TpmReady",
            ],
            dig_deeper: Some("bitlocker"),
        },
    },

    // ── DNS resolution failing ────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["dns resolution: failed", "dns: failed", "dns fail", "dns resolution failed", "could not resolve"],
        recipe: Recipe {
            severity: "ACTION",
            title: "DNS resolution failing",
            steps: &[
                "Flush DNS cache: PowerShell (admin) → Clear-DnsClientCache",
                "Test DNS directly: PowerShell → Resolve-DnsName google.com -Server 8.8.8.8 — if this works, your DNS server is the problem",
                "Switch to a reliable DNS server: PowerShell (admin) → Set-DnsClientServerAddress -InterfaceAlias 'Wi-Fi' -ServerAddresses ('8.8.8.8','1.1.1.1')",
                "Check if the DNS client service is running: Get-Service Dnscache | Select Status",
                "If on a corporate network or VPN, contact IT — split DNS may require the VPN to be connected for internal names to resolve",
            ],
            dig_deeper: Some("dns_servers"),
        },
    },

    // ── Repeated app crashes ──────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["faulting application", "crash count", "crash frequency", "application hang", "faulting module"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Application crashing repeatedly",
            steps: &[
                "Note the faulting application name and module from the report — these are the most important clues",
                "If the faulting module is ntdll.dll or a system DLL: run SFC to repair Windows files → PowerShell (admin) → sfc /scannow",
                "If the faulting module is a third-party DLL (e.g. a codec or plugin): uninstall the associated program",
                "Update or reinstall the crashing application — corrupted installs are a common cause",
                "Check for conflicting software: antivirus, screen recorders, and overlays (Discord, GeForce Experience) frequently inject into other processes",
                "If it is a Microsoft Office app: run the Office repair → Control Panel → Programs → right-click Office → Change → Quick Repair",
            ],
            dig_deeper: Some("app_crashes"),
        },
    },

    // ── Visual C++ / runtime missing ─────────────────────────────────────────
    RecipeEntry {
        triggers: &["vcruntime", "msvcr", "0xc000007b", "side-by-side configuration", "missing runtime", "vc++ redistributable"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Visual C++ runtime missing or corrupt",
            steps: &[
                "Download and install the latest Visual C++ Redistributable packages (both x64 and x86) from Microsoft: search 'Visual C++ Redistributable downloads'",
                "Install all available years: 2015–2022 package covers most apps; older apps may need 2013, 2012, or 2010 separately",
                "If a specific app shows error 0xc000007b: right-click the app → Properties → Compatibility → Run as administrator",
                "Repair existing runtimes: Control Panel → Programs → find 'Microsoft Visual C++ 20XX' → Repair",
                "After installing, restart before testing the application again — runtimes must be registered at boot",
            ],
            dig_deeper: None,
        },
    },

    // ── Certificate expiring ──────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["expiring within 30 days", "expires in", "certificate expir", "cert.*expir"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Certificate expiring soon",
            steps: &[
                "Open Certificate Manager: press Win+R → type 'certmgr.msc' → check Personal → Certificates for the expiring cert",
                "Note the certificate subject and issuer — determines who you need to contact for renewal",
                "For personal/S-MIME certificates: renew through your CA or email provider portal",
                "For web/TLS certificates on a server: generate a new CSR and submit to your CA before expiry",
                "For code-signing certificates: do not let these lapse — signed binaries will show 'unknown publisher' warnings after expiry",
            ],
            dig_deeper: Some("certificates"),
        },
    },

    // ── Wi-Fi weak signal ─────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["signal: poor", "weak signal", "rssi: -8", "rssi: -9", "signal strength: poor", "quality: poor", "poor signal"],
        recipe: Recipe {
            severity: "MONITOR",
            title: "Wi-Fi signal weak",
            steps: &[
                "Move closer to the router or access point — Wi-Fi degrades quickly through walls and floors",
                "Switch to 5 GHz band if available — faster and less congested in most home environments (but shorter range than 2.4 GHz)",
                "Check for interference: microwave ovens, baby monitors, and neighboring networks on the same channel all degrade signal",
                "Change the router's Wi-Fi channel: log into router admin → Wireless settings → try channels 1, 6, or 11 (2.4 GHz) or auto (5 GHz)",
                "Update the Wi-Fi adapter driver: Device Manager → Network Adapters → right-click adapter → Update driver",
                "If signal is consistently poor from a fixed desk, consider a powerline adapter or mesh Wi-Fi node nearby",
            ],
            dig_deeper: Some("wifi"),
        },
    },

    // ── NTP / time sync failure ───────────────────────────────────────────────
    RecipeEntry {
        triggers: &["time sync failed", "sync failed", "clock drift", "ntp.*error", "w32tm.*fail", "ntp source.*unreachable", "time.*not synchronized"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "System clock not synchronizing",
            steps: &[
                "Force a sync now: PowerShell (admin) → w32tm /resync /force",
                "Check the current NTP source: PowerShell → w32tm /query /source",
                "If source shows 'Local CMOS Clock' or 'Free-running', the time service has lost its server",
                "Reset to Microsoft's NTP server: PowerShell (admin) → w32tm /config /manualpeerlist:time.windows.com /syncfromflags:manual /reliable:YES /update",
                "Restart the time service: PowerShell (admin) → Restart-Service w32tm",
                "If clock drift is large (>5 minutes), some authentication systems (Kerberos, MFA) will fail until synced",
            ],
            dig_deeper: Some("ntp"),
        },
    },

    // ── Page file missing ─────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["no page file", "pagefile: none", "page file: none", "virtual memory: none", "pagefile not configured", "no pagefile"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Page file not configured",
            steps: &[
                "Windows needs a page file even with plenty of RAM — some apps and crash dumps require it",
                "Re-enable automatic page file management: search 'Adjust the appearance and performance of Windows' → Advanced → Virtual memory → Change → check 'Automatically manage'",
                "If manually set: assign at least 1.5× your RAM as maximum size on the system drive",
                "After changing page file settings, restart is required — changes do not take effect until reboot",
                "Note: if this machine intentionally has no page file (e.g. a RAM disk setup), verify that was deliberate before changing it",
            ],
            dig_deeper: Some("pagefile"),
        },
    },

    // ── System file corruption ────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["corrupt files found", "autorepairrequired: true", "integrity.*failed", "component store corruption", "sfc.*corrupt", "windows resource protection found corrupt"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Windows system file corruption detected",
            steps: &[
                "Run SFC to repair corrupt files: PowerShell (admin) → sfc /scannow (takes 5–15 minutes)",
                "If SFC reports 'Windows Resource Protection found corrupt files but was unable to fix some of them', run DISM next:",
                "DISM repair: PowerShell (admin) → DISM /Online /Cleanup-Image /RestoreHealth (requires internet access, 10–30 minutes)",
                "Run SFC again after DISM completes — DISM provides the source files SFC needs",
                "Restart after both complete, then check Event Viewer for CBS log: Applications and Services Logs → Microsoft → Windows → Servicing",
                "If corruption persists after both tools: in-place upgrade repair (Windows Setup without wiping data) is the next step",
            ],
            dig_deeper: Some("integrity"),
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
