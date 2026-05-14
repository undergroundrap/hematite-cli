use std::fmt::Write as _;

pub struct Recipe {
    pub severity: &'static str,
    pub title: &'static str,
    pub steps: &'static [&'static str],
    pub dig_deeper: Option<&'static str>,
}

struct RecipeAc {
    ac: aho_corasick::AhoCorasick,
    recipe_indices: Vec<usize>,
}

static RECIPE_AC: std::sync::OnceLock<RecipeAc> = std::sync::OnceLock::new();

fn recipe_ac() -> &'static RecipeAc {
    RECIPE_AC.get_or_init(|| {
        let total: usize = ALL_RECIPES.iter().map(|e| e.triggers.len()).sum();
        let mut patterns: Vec<&str> = Vec::with_capacity(total);
        let mut recipe_indices: Vec<usize> = Vec::with_capacity(total);
        for (i, entry) in ALL_RECIPES.iter().enumerate() {
            for &trigger in entry.triggers {
                patterns.push(trigger);
                recipe_indices.push(i);
            }
        }
        RecipeAc {
            ac: aho_corasick::AhoCorasick::new(&patterns).expect("valid patterns"),
            recipe_indices,
        }
    })
}

pub fn match_recipes(output: &str) -> Vec<&'static Recipe> {
    let lower = output.to_ascii_lowercase();
    let state = recipe_ac();
    let mut seen = std::collections::HashSet::new();
    let mut matches: Vec<&'static Recipe> = Vec::new();
    for mat in state.ac.find_iter(&lower) {
        let idx = state.recipe_indices[mat.pattern().as_usize()];
        if seen.insert(idx) {
            matches.push(&ALL_RECIPES[idx].recipe);
        }
    }
    matches
}

fn collect_unique_recipes(outputs: &[(&str, &str)]) -> Vec<&'static Recipe> {
    let mut all_recipes: Vec<&'static Recipe> = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();
    for (_label, output) in outputs {
        for recipe in match_recipes(output) {
            if seen_titles.insert(recipe.title) {
                all_recipes.push(recipe);
            }
        }
    }
    all_recipes
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

    // ── Service start failure ─────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["stopped unexpectedly", "failed to start", "error 1067", "error 1053", "service terminated", "exited with code", "failed to respond"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Service failed to start or stopped unexpectedly",
            steps: &[
                "Find the failing service name in the report, then check its status: PowerShell → Get-Service <ServiceName>",
                "Read the specific error from the Application/System event log: Event Viewer → Windows Logs → System → filter for Service Control Manager (Event ID 7034 or 7031)",
                "Try to start it manually: PowerShell (admin) → Start-Service <ServiceName> — note any error message",
                "Check if the service account has the right permissions: Services console (services.msc) → right-click → Properties → Log On tab",
                "Look for a dependent service that failed first — a service won't start if something it requires is stopped",
                "If the service EXE is missing or corrupt, reinstall the application that owns it",
            ],
            dig_deeper: Some("services"),
        },
    },

    // ── RDP unreachable ───────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["fdenytsconnections: 1", "no enabled rdp firewall", "rdp status: disabled"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Remote Desktop (RDP) is disabled or blocked",
            steps: &[
                "Enable RDP: Settings → System → Remote Desktop → Enable Remote Desktop (or PowerShell admin: Set-ItemProperty 'HKLM:\\System\\CurrentControlSet\\Control\\Terminal Server' fDenyTSConnections 0)",
                "Ensure the RDP firewall rule is enabled: PowerShell (admin) → Enable-NetFirewallRule -DisplayGroup 'Remote Desktop'",
                "Verify port 3389 is listening after enabling: PowerShell → netstat -an | findstr 3389",
                "If NLA is required, make sure the connecting user account has the right to log in remotely (must be in Remote Desktop Users group or Administrators)",
                "Check that Windows Firewall is not blocking the connection — on the host, temporarily allow pings to confirm network path is open",
                "For cloud VMs: check the security group / NSG allows inbound TCP 3389 from your IP",
            ],
            dig_deeper: Some("rdp"),
        },
    },

    // ── Windows Update service broken ─────────────────────────────────────────
    RecipeEntry {
        triggers: &["wuauserv: stopped", "wuauserv stopped", "windows update: stopped", "update service stopped", "bits: stopped", "bits stopped"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Windows Update service is stopped or broken",
            steps: &[
                "Run the Windows Update Troubleshooter: Settings → Update & Security → Troubleshoot → Additional troubleshooters → Windows Update",
                "Manually restart the update services: PowerShell (admin) → Stop-Service wuauserv, bits, cryptsvc, msiserver → Start-Service wuauserv, bits, cryptsvc",
                "Clear the update cache if stuck: PowerShell (admin) → Stop-Service wuauserv → Remove-Item C:\\Windows\\SoftwareDistribution\\* -Recurse -Force → Start-Service wuauserv",
                "Check for conflicting 3rd-party update tools (WSUS, SCCM, Intune policies) that may be disabling updates",
                "Run the System Update Readiness Tool: DISM /Online /Cleanup-Image /RestoreHealth",
                "If the service keeps stopping, check Event Viewer → Windows Logs → System for Windows Update Agent errors around the same time",
            ],
            dig_deeper: Some("updates"),
        },
    },

    // ── Teams cache ───────────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["classic teams cache:", "new teams cache:", "msteams cache:", "teams cache size"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Teams cache — clear to resolve most Teams issues",
            steps: &[
                "Quit Teams completely: right-click the Teams icon in the system tray → Quit",
                "Clear Classic Teams cache: open Run (Win+R) → type %AppData%\\Microsoft\\Teams → delete the contents of: Cache, blob_storage, databases, GPUCache, IndexedDB, Local Storage, tmp",
                "Clear New Teams (MSTeams) cache: open Run → %LocalAppData%\\Packages\\MSTeams_8wekyb3d8bbwe\\LocalCache\\ → delete all contents",
                "Restart Teams and sign in — cache rebuilds from the server automatically",
                "If Teams still fails to sign in after clearing cache, also clear credentials: Credential Manager (Win+R → credmgr.msc) → Windows Credentials → remove all MicrosoftOffice16_Data:SSPI:* entries",
            ],
            dig_deeper: Some("teams"),
        },
    },

    // ── M365 token broker not running ─────────────────────────────────────────
    RecipeEntry {
        triggers: &["token broker: not running", "aad broker plugin: not found", "web account manager: not running", "wam: not running", "aad broker: not found"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Microsoft 365 authentication broker not running",
            steps: &[
                "The Windows Account Manager (WAM) and AAD Broker are required for M365 sign-in — if they're not running, Teams, Outlook, and OneDrive will loop on sign-in",
                "Re-register the token broker: PowerShell (admin) → sfc /scannow — this repairs the system files WAM depends on",
                "Restart the TokenBroker service: PowerShell (admin) → Restart-Service TokenBroker -ErrorAction SilentlyContinue",
                "If re-registering doesn't help, sign out of all work accounts: Settings → Accounts → Access work or school → disconnect and reconnect your org account",
                "On Intune/AAD-joined machines: run 'dsregcmd /leave' then 'dsregcmd /join' (admin) to re-register the device — requires network connectivity to Azure AD",
                "Check for conflicting credential entries: Credential Manager → Windows Credentials → remove stale MicrosoftOffice16_Data:SSPI:* and MicrosoftOffice15_Data:* entries",
            ],
            dig_deeper: Some("identity_auth"),
        },
    },

    // ── WMI repository corrupt ────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["wmi repository is inconsistent", "repository is inconsistent", "wmi: inconsistent", "verifyrepository: inconsistent", "wmi corruption"],
        recipe: Recipe {
            severity: "ACTION",
            title: "WMI repository corrupt — cascading tool failures",
            steps: &[
                "WMI corruption breaks PowerShell Get-WmiObject, Defender, Windows Update, and many admin tools — fix it first before investigating other issues",
                "Stop WMI: PowerShell (admin) → net stop winmgmt /y",
                "Rebuild the repository: PowerShell (admin) → winmgmt /resetrepository",
                "Start WMI: PowerShell (admin) → net start winmgmt",
                "Verify the fix: PowerShell → winmgmt /verifyrepository — should say 'WMI repository is consistent'",
                "If resetrepository fails, try salvage mode: winmgmt /salvagerepository — this preserves customizations",
                "Restart the machine after repair — WMI caches are session-scoped and some tools won't see the fix until reboot",
            ],
            dig_deeper: Some("wmi_health"),
        },
    },

    // ── Windows not activated ─────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["license status: unlicensed", "license status: notification", "activation: not activated", "not genuine", "windows is not activated"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Windows not activated",
            steps: &[
                "Check activation status: Settings → System → Activation — note the exact status message",
                "If you have a product key: Settings → System → Activation → Change product key → enter the 25-character key",
                "If the key was tied to a Microsoft account: sign in with that Microsoft account and activation should happen automatically over the internet",
                "Force activation attempt: PowerShell (admin) → slmgr /ato",
                "If you get error 0xC004F074 (Key Management Service unreachable): you're on a domain with KMS — contact your IT department, the KMS server may be offline",
                "If you recently changed hardware (motherboard): activation may need to be relinked — use the Activation Troubleshooter in Settings",
            ],
            dig_deeper: Some("activation"),
        },
    },

    // ── Windows Search not indexing ───────────────────────────────────────────
    RecipeEntry {
        triggers: &["wsearch: stopped", "search service: stopped", "wsearch service: stopped", "indexer: stopped", "windows search: stopped"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Windows Search not running — search won't find files",
            steps: &[
                "Start the Windows Search service: PowerShell (admin) → Start-Service WSearch",
                "Set it to start automatically: PowerShell (admin) → Set-Service WSearch -StartupType Automatic",
                "If the service won't start: check Event Viewer → Windows Logs → Application → filter for 'Search' for the specific error",
                "Rebuild the search index: Settings → Privacy & Security → Windows Search → Advanced indexing options → Advanced → Rebuild — takes 15–60 minutes",
                "If rebuilding doesn't help, reset the index database: Stop-Service WSearch → delete C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb → Start-Service WSearch",
                "Restart File Explorer after: PowerShell → Stop-Process -Name explorer → Start-Process explorer",
            ],
            dig_deeper: Some("search_index"),
        },
    },

    // ── OneDrive sync error ───────────────────────────────────────────────────
    RecipeEntry {
        triggers: &["sync status: error", "onedrive: not running", "sync errors detected", "onedrive sync error", "known folder backup: not configured"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "OneDrive not syncing",
            steps: &[
                "Check the sync status icon in the system tray — hover over it for the specific error message",
                "Common fix: right-click the OneDrive tray icon → Pause syncing → Resume syncing — resets stuck sync state",
                "If that doesn't work: right-click the OneDrive tray icon → Settings → Account → Unlink this PC → relink with the same account",
                "Check for conflicting files: File Explorer → OneDrive folder → look for files with a red X — rename or delete the local copy and let it sync from the cloud",
                "If the issue is 'Not enough space in OneDrive': manage storage at onedrive.live.com/manage",
                "Reset OneDrive if all else fails: Win+R → %localappdata%\\Microsoft\\OneDrive\\onedrive.exe /reset — wait 2 minutes, then reopen OneDrive from Start",
            ],
            dig_deeper: Some("onedrive"),
        },
    },

    // ── Printer offline or stuck queue ────────────────────────────────────────
    RecipeEntry {
        triggers: &["status: offline", "pending jobs:", "print spooler: stopped", "spooler: stopped"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Printer offline or stuck print queue",
            steps: &[
                "Check the printer is powered on and connected (USB cable or same Wi-Fi network as the PC)",
                "Clear the stuck print queue: PowerShell (admin) → Stop-Service Spooler → Remove-Item C:\\Windows\\System32\\spool\\PRINTERS\\* -Force → Start-Service Spooler",
                "If printer shows Offline: right-click the printer in Settings → Printers & scanners → See what's printing → Printer menu → uncheck 'Use Printer Offline'",
                "For network printers: verify the printer's IP hasn't changed — print a configuration page from the printer itself to check its current IP",
                "Re-add the printer if the IP changed: Settings → Bluetooth & devices → Printers & scanners → Add device → Add manually → enter the new IP",
                "If the Print Spooler service is stopped: PowerShell (admin) → Start-Service Spooler → Set-Service Spooler -StartupType Automatic",
            ],
            dig_deeper: Some("printers"),
        },
    },

    // ── No Outlook mail profile ───────────────────────────────────────────────
    RecipeEntry {
        triggers: &["profile count: 0", "no mail profiles", "mail profile: none", "no profiles configured", "outlook profiles: 0"],
        recipe: Recipe {
            severity: "ACTION",
            title: "No Outlook mail profile — Outlook will not open",
            steps: &[
                "Outlook requires at least one mail profile to start — create one from the Mail control panel applet, not from within Outlook",
                "Open Mail applet: Win+R → type 'control mlcfg32.cpl' (or search 'Mail' in Control Panel) → Show Profiles → Add",
                "Enter a profile name (e.g. 'Outlook') → Add Account → enter your email address and follow the auto-configuration wizard",
                "For Exchange/Microsoft 365: the wizard needs network access to find the Autodiscover DNS record — ensure VPN is connected if this is a corporate account",
                "For manual setup: choose 'Manual setup' → Microsoft Exchange or compatible service → enter server and username from your IT department",
                "After creating the profile: launch Outlook, sign in if prompted — first launch will take 2–10 minutes to download the mailbox",
            ],
            dig_deeper: Some("outlook"),
        },
    },

    // ── PrintNightmare not mitigated ──────────────────────────────────────────
    RecipeEntry {
        triggers: &["rpcauthnlevelprivacyenabled: 0", "printnightmare rpc mitigation not applied", "point and print allows silent", "finding: printnightmare"],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "PrintNightmare (CVE-2021-34527) mitigation not applied",
            steps: &[
                "Apply the RPC authentication hardening fix: PowerShell (admin) → Set-ItemProperty 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\Print' -Name RpcAuthnLevelPrivacyEnabled -Value 1 -Type DWord",
                "Restrict Point and Print driver installs to administrators: PowerShell (admin) → Set-ItemProperty 'HKLM:\\SOFTWARE\\Policies\\Microsoft\\Windows NT\\Printers\\PointAndPrint' -Name RestrictDriverInstallationToAdministrators -Value 1 -Type DWord",
                "If the Print Spooler is not needed (e.g. server, workstation that never prints remotely): PowerShell (admin) → Stop-Service Spooler → Set-Service Spooler -StartupType Disabled",
                "Verify patch KB5004945 or later is installed: check Windows Update history for July 2021 security updates",
                "Restart the Spooler service after registry changes: PowerShell (admin) → Restart-Service Spooler",
            ],
            dig_deeper: Some("print_spooler"),
        },
    },

    // ── TCP/IP stack corruption ───────────────────────────────────────────────
    RecipeEntry {
        triggers: &["winsock catalog corrupted", "winsock reset", "tcp/ip stack", "netsh int ip reset", "no internet after update", "network stack corrupt"],
        recipe: Recipe {
            severity: "ACTION",
            title: "TCP/IP or Winsock stack needs reset",
            steps: &[
                "Open PowerShell as administrator",
                "Reset the TCP/IP stack: netsh int ip reset",
                "Reset the Winsock catalog: netsh winsock reset",
                "Restart the computer — these changes require a reboot to take effect",
                "After restart, verify internet is restored: ping 1.1.1.1",
                "If still broken, run: ipconfig /release then ipconfig /renew to get a fresh DHCP lease",
            ],
            dig_deeper: Some("connectivity"),
        },
    },

    // ── WLAN AutoConfig service stopped ──────────────────────────────────────
    RecipeEntry {
        triggers: &["wlansvc: stopped", "wlan autoconfig: stopped", "wlan autoconfig service: stopped", "wireless autoconfig: stopped"],
        recipe: Recipe {
            severity: "ACTION",
            title: "WLAN AutoConfig service stopped — Wi-Fi unavailable",
            steps: &[
                "Open PowerShell as administrator",
                "Start the WLAN AutoConfig service: Start-Service Wlansvc",
                "Set it to auto-start: Set-Service Wlansvc -StartupType Automatic",
                "Verify Wi-Fi adapter is visible: Get-NetAdapter | Where-Object {$_.MediaType -eq '802.11'}",
                "If no Wi-Fi adapter appears after starting the service, check Device Manager for a disabled wireless adapter",
                "If the service fails to start, update the Wi-Fi adapter driver via Device Manager → right-click adapter → Update driver",
            ],
            dig_deeper: Some("wifi"),
        },
    },

    // ── BSOD / unexpected shutdown ────────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "system crashes / unexpected shutdowns:",
            "bsod (bugcheck)",
            "unexpected shutdown",
            "blue screen",
            "stop code",
            "critical_process_died",
            "memory_management",
            "kernel_security_check_failure",
            "page_fault_in_nonpaged_area",
            "irql_not_less_or_equal",
            "system_thread_exception",
            "kmode_exception_not_handled",
            "ntfs_file_system",
            "bsod after",
            "keeps crashing",
            "random restart",
            "random reboot",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "Blue screen (BSOD) or unexpected shutdown",
            steps: &[
                "Find the stop code: Settings → System → About → Blue screen of death stop code, or check Event Viewer (Win+X → Event Viewer → Windows Logs → System → filter for Event ID 41 and 1001)",
                "Run memory diagnostics: Win+R → type 'mdsched' → restart and test now — leave it running overnight if possible; replace RAM if errors found",
                "Check for corrupted system files: PowerShell (admin) → sfc /scannow, then DISM /Online /Cleanup-Image /RestoreHealth (takes 15–30 min, requires internet)",
                "Roll back recently installed drivers: Device Manager → look for recently updated drivers (sort by date) → right-click → Properties → Driver → Roll Back Driver",
                "Check for Windows Update issues: some BSODs after updates are fixed by the next cumulative update — check Windows Update for pending updates",
                "For memory-related stop codes (MEMORY_MANAGEMENT, PAGE_FAULT_IN_NONPAGED_AREA): reseat RAM sticks (power off, remove and firmly reinsert each stick)",
                "Read the minidump for the exact faulting module: PowerShell (admin) → Get-ChildItem C:\\Windows\\Minidump | Sort-Object LastWriteTime -Descending | Select-Object -First 3 FullName — open with WinDbg or upload to https://www.osronline.com/page.cfm?name=analyze",
                "If BSODs persist: run hematite --inspect disk_health,thermal to rule out failing storage or overheating as the root cause",
            ],
            dig_deeper: Some("recent_crashes"),
        },
    },

    // ── Webcam / camera not working ───────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "global: deny",
            "camera access is globally denied",
            "no camera devices found via pnp",
            "camera not working",
            "webcam not working",
            "camera not detected",
            "camera blocked",
            "camera privacy",
            "no cameras found",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "Camera / webcam not working or blocked",
            steps: &[
                "Check Windows camera privacy first: Settings → Privacy & security → Camera → toggle 'Let apps access your camera' ON",
                "If the global toggle is already on, check the specific app (Teams, Zoom, etc.) — scroll down on the same page and enable it for the app individually",
                "Test the camera works at all: Win+R → type 'camera' → open the Camera app — if it works there, the issue is app-specific permissions, not the device",
                "If Camera app also fails: open Device Manager (Win+X) → Cameras — look for a yellow bang (error) on the device, right-click → Update driver → Search automatically",
                "If no camera appears in Device Manager at all: check if the camera is physically disabled via a keyboard shortcut (often Fn+F key with a camera icon), or disabled in BIOS/UEFI",
                "Reinstall the camera driver: Device Manager → Cameras → right-click → Uninstall device (check 'Delete driver' box) → restart PC; Windows re-installs the driver on boot",
                "For external USB cameras: try a different USB port or USB cable; test on another machine to confirm the hardware is not faulty",
                "In Teams or Zoom: check in-app settings → Video/Camera device selector — make sure the correct camera is selected (especially if multiple cameras exist)",
            ],
            dig_deeper: Some("camera"),
        },
    },

    // ── Windows Firewall service stopped ─────────────────────────────────────
    RecipeEntry {
        triggers: &["firewall service: stopped", "mpssvc: stopped", "windows firewall service: stopped", "firewall: stopped"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Windows Firewall service stopped — security risk",
            steps: &[
                "Open PowerShell as administrator",
                "Start the Windows Firewall service: Start-Service MpsSvc",
                "Set it to auto-start: Set-Service MpsSvc -StartupType Automatic",
                "Verify all profiles are active: Get-NetFirewallProfile | Select Name, Enabled",
                "If MpsSvc fails to start, check for third-party firewall software that may have disabled it",
                "In an enterprise environment, the firewall may be managed by Group Policy — run gpresult /r to check",
            ],
            dig_deeper: Some("security"),
        },
    },

    // ── No audio / sound not working ─────────────────────────────────────────
    RecipeEntry {
        triggers: &["core audio services are not running", "no audio endpoints", "audio service not running", "audiosrv: not running", "no playback devices", "no sound device"],
        recipe: Recipe {
            severity: "ACTION",
            title: "No audio — Windows Audio service or device issue",
            steps: &[
                "Restart the Windows Audio service: PowerShell (admin) → Restart-Service Audiosrv -Force",
                "If the service restarts but no sound, check Device Manager for audio device errors: Win+X → Device Manager → Sound, video and game controllers",
                "Right-click any device with a yellow bang → Update driver → Search automatically",
                "Check the audio device is not muted or disabled: right-click volume icon in taskbar → Open Sound settings → check Output device",
                "For Bluetooth headsets: run hematite --inspect bluetooth to check the AVCTP service and audio crossover state",
                "If using a USB audio device: unplug and replug the device, or try a different USB port",
                "Last resort — reinstall the audio driver: Device Manager → Sound → right-click device → Uninstall device → Restart PC (Windows re-installs the driver)",
            ],
            dig_deeper: Some("audio"),
        },
    },

    // ── Bluetooth pairing/connection issues ──────────────────────────────────
    RecipeEntry {
        triggers: &["bluetooth-related services are not fully running", "no bluetooth radio", "bluetooth device issues", "bluetooth adapter not detected", "bthserv: stopped", "bluetooth service not running"],
        recipe: Recipe {
            severity: "ACTION",
            title: "Bluetooth not working — service or hardware issue",
            steps: &[
                "Restart the Bluetooth services: PowerShell (admin) → Restart-Service bthserv -Force",
                "If the service starts but devices won't pair: toggle Bluetooth off then on in Settings → Bluetooth & devices",
                "Remove the device and re-pair it: Settings → Bluetooth & devices → find the device → Remove → Add device",
                "If no Bluetooth adapter is detected: check Device Manager → Bluetooth — look for missing or disabled adapters",
                "For a disabled adapter: right-click → Enable device; for a missing adapter, check if Bluetooth is disabled in BIOS/UEFI or via a physical switch/key combination",
                "Update the Bluetooth driver: Device Manager → Bluetooth → right-click adapter → Update driver",
                "For Bluetooth audio: verify the device is set as the default audio output in Settings → System → Sound",
            ],
            dig_deeper: Some("bluetooth"),
        },
    },

    // ── Windows Installer / app installation failing ──────────────────────────
    RecipeEntry {
        triggers: &["msiserver) is disabled", "msiexec.exe is missing", "installer transaction already in progress", "recent installer failures were recorded", "microsoft desktop app installer is missing", "msiserver: disabled"],
        recipe: Recipe {
            severity: "ACTION",
            title: "App installation failing — Windows Installer issue",
            steps: &[
                "Re-enable and start the Windows Installer service: PowerShell (admin) → Set-Service msiserver -StartupType Manual; Start-Service msiserver",
                "If an installer transaction is stuck, clear it: PowerShell (admin) → msiexec /unreg; msiexec /regserver",
                "If a pending reboot is blocking installs: save your work and restart the computer first",
                "For winget install failures: ensure Microsoft Desktop App Installer is current — open Microsoft Store → search 'App Installer' → Update",
                "Clear the Windows Installer temp files: delete C:\\Windows\\Installer\\*.tmp",
                "If MSI installs still fail after the above, run: sfc /scannow in an admin PowerShell to repair system files that MSI depends on",
            ],
            dig_deeper: Some("installer_health"),
        },
    },

    // ── VPN not connecting ────────────────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "vpn adapter detected",
            "no vpn adapters found",
            "vpn client service",
            "vpn tunnel",
            "vpn not connecting",
            "vpn disconnecting",
            "split tunnel",
            "ras/vpn",
            "rasman",
        ],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "VPN not connecting or disconnecting",
            steps: &[
                "Restart the Remote Access Connection Manager service: PowerShell (admin) → Restart-Service RasMan -Force",
                "Check for a proxy conflict: Settings → Network & Internet → Proxy — disable 'Automatically detect settings' temporarily and test again",
                "Flush DNS and reset Winsock (VPN can leave stale routes): PowerShell (admin) → ipconfig /flushdns; netsh winsock reset",
                "If using a corporate VPN client (Cisco AnyConnect, GlobalProtect, Pulse): reinstall the client or run the client's repair option from Add/Remove Programs",
                "Check Windows Firewall isn't blocking the VPN ports: run hematite --inspect firewall_rules and look for rules blocking UDP 500, UDP 4500, or TCP 1723",
                "If the VPN adapter shows but won't authenticate: run hematite --inspect identity_auth to check if the AAD/WAM token broker is healthy",
                "For WireGuard or OpenVPN: verify the tunnel config file is intact and the remote server is reachable: ping <vpn-server-ip>",
            ],
            dig_deeper: Some("vpn"),
        },
    },

    // ── Screen flickering / display issues ───────────────────────────────────
    RecipeEntry {
        triggers: &[
            "display driver",
            "refresh rate:",
            "bits per pixel:",
            "monitor:",
            "screen flickering",
            "display flickering",
            "screen flashing",
            "black screen",
            "resolution wrong",
            "wrong resolution",
        ],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Screen flickering or display issues",
            steps: &[
                "Update or roll back the display driver: Device Manager → Display Adapters → right-click GPU → Update driver (or Roll Back Driver if flickering started after a recent update)",
                "Check for a refresh rate mismatch: Settings → System → Display → Advanced display → verify the refresh rate matches what your monitor supports",
                "Inspect the cable: reseat or replace the HDMI/DisplayPort cable — a loose or failing cable is the most common cause of flickering",
                "Check if Task Manager flickers when you open it (Win+X → Task Manager): if Task Manager does NOT flicker, the cause is a software/app conflict, not the driver",
                "Disable hardware acceleration in Chrome/Edge: Settings → System → turn off 'Use hardware acceleration when available', then restart the browser",
                "Run hematite --inspect device_health to check for GPU or display adapter PnP errors (yellow bangs in Device Manager)",
                "If using NVIDIA: open NVIDIA Control Panel → Manage 3D Settings → verify G-Sync/VRR is correctly enabled or disabled for your panel type",
                "For laptops: test on an external monitor — if the external is stable, the laptop panel or cable is the likely failure point",
            ],
            dig_deeper: Some("display_config"),
        },
    },

    // ── Microphone not working ────────────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "no recording endpoints found",
            "recording endpoint",
            "microphone privacy",
            "microphone access: denied",
            "input device",
            "microphone not working",
            "mic not working",
            "mic not detected",
            "microphone blocked",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "Microphone not working or not detected",
            steps: &[
                "Check the privacy setting: Settings → Privacy & security → Microphone → ensure 'Microphone access' is On and your app has permission",
                "Set the correct default input device: right-click the speaker icon in the taskbar → Sound settings → Input → choose the correct microphone",
                "Restart the Windows Audio service: PowerShell (admin) → Restart-Service Audiosrv -Force; Restart-Service AudioEndpointBuilder -Force",
                "Check the microphone is not muted at the hardware level — look for a physical mute button on the mic, headset, or laptop keyboard (Fn+F-key)",
                "Run hematite --inspect audio to see which recording endpoints Windows has found and their current state",
                "Update the audio driver: Device Manager → Sound, video and game controllers → right-click the audio device → Update driver",
                "For a USB microphone: unplug, wait 10 seconds, replug — Windows re-enumerates the device and may fix a bad enumeration state",
                "If the microphone works in one app but not another (e.g. works in Voice Recorder but not Teams): check the app's own audio settings, not Windows settings",
            ],
            dig_deeper: Some("audio"),
        },
    },

    // ── Login / PIN / Windows Hello not working ───────────────────────────────
    RecipeEntry {
        triggers: &[
            "wbiosrvc",
            "biometric service",
            "windows hello",
            "pin not working",
            "fingerprint not working",
            "logon failure",
            "event id 4625",
            "failed logon",
            "credential provider",
            "sign-in failed",
            "can't sign in",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "Login, PIN, or Windows Hello not working",
            steps: &[
                "Reset the PIN: on the sign-in screen, click 'I forgot my PIN' — Windows will verify your Microsoft account or Azure AD identity and let you set a new PIN without needing the old one",
                "Restart the Windows Biometric Service: PowerShell (admin) → Restart-Service WbioSrvc -Force",
                "If fingerprint or face recognition stopped working: Settings → Accounts → Sign-in options → remove and re-enroll the biometric credential",
                "For 'something went wrong' on the PIN screen: Settings → Accounts → Sign-in options → PIN (Windows Hello) → I forgot my PIN",
                "Check for account lockout (repeated failed logins): run hematite --inspect sign_in and look for Event ID 4625 (failed logon) frequency",
                "If the machine is domain-joined and the domain controller is unreachable, Windows may not accept domain credentials — use a local admin account as a fallback",
                "If all sign-in methods fail at the lock screen, boot to Windows Recovery (hold Shift while clicking Restart) → Troubleshoot → Reset this PC as a last resort",
            ],
            dig_deeper: Some("sign_in"),
        },
    },

    // ── High disk I/O — disk at 100% ─────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "disk queue length:",
            "average disk queue",
            "disk i/o",
            "high disk usage",
            "disk at 100",
            "100% disk",
            "disk thrashing",
            "disk saturation",
        ],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Disk at 100% — high disk I/O",
            steps: &[
                "Identify the process driving disk I/O: run hematite --inspect processes and look at the R/W column for the top consumer",
                "Common culprits: Windows Search indexer (SearchIndexer.exe), Windows Update (TiWorker.exe, WUDFHost.exe), Antivirus scan, or a runaway backup job",
                "If Windows Search is the cause: Settings → Search → Windows Search → Indexing Options → Pause indexing for 15 minutes and see if disk drops",
                "If Windows Update (TiWorker.exe): let it finish — fighting a mid-update installation makes things worse; check Windows Update status in Settings",
                "Check for a failing drive: run hematite --inspect disk_health — a drive with SMART errors can cause 100% disk usage as the OS retries failing sectors",
                "Disable Windows Superfetch/SysMain if you have an SSD: PowerShell (admin) → Stop-Service SysMain; Set-Service SysMain -StartupType Disabled",
                "Check the page file: if RAM is full, Windows pages to disk constantly — run hematite --inspect pagefile and resource_load to verify",
                "For persistent 100% disk on an HDD: consider upgrading to an SSD — HDDs cannot sustain the I/O demand of modern Windows workloads",
            ],
            dig_deeper: Some("processes"),
        },
    },

    // ── USB device not recognized ─────────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "[err:",
            "usb device not recognized",
            "unknown usb device",
            "device descriptor request failed",
            "usb not working",
            "usb port not working",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "USB device not recognized or not working",
            steps: &[
                "Unplug the USB device, wait 10 seconds, then replug it — Windows re-enumerates the USB controller and often clears a bad enumeration state",
                "Try a different USB port — especially try a USB 2.0 port if the device was on a USB 3.0 port (some older accessories have compatibility issues with 3.x)",
                "Restart the USB Host Controller: Device Manager → Universal Serial Bus controllers → right-click each 'USB Root Hub' → Disable device, then Enable device",
                "Run the USB troubleshooter: Settings → System → Troubleshoot → Other troubleshooters → USB",
                "Check for driver errors: run hematite --inspect device_health to see PnP error codes — Error Code 43 is an unrecoverable device error requiring a driver reinstall or device replacement",
                "Update chipset and USB controller drivers: go to your motherboard manufacturer's website (ASUS, MSI, Gigabyte, ASRock) and download the latest chipset driver for your platform",
                "If the device worked before: uninstall the device in Device Manager (right-click → Uninstall device, check 'Delete the driver software'), then replug so Windows installs a fresh driver",
                "For persistent issues on all ports: run hematite --inspect device_health and check if 'USB Root Hub' itself shows an error — this points to a chipset/driver issue, not the accessory",
            ],
            dig_deeper: Some("device_health"),
        },
    },

    // ── No Wi-Fi networks visible / can't find networks ───────────────────────
    RecipeEntry {
        triggers: &[
            "there is no wireless interface",
            "no wireless interface detected",
            "no wi-fi devices found",
            "wi-fi adapter disconnected",
            "no wireless tool available",
            "no wireless networks",
            "wifi adapter off",
            "wireless adapter disabled",
            "airplane mode",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "No Wi-Fi networks visible — adapter or driver issue",
            steps: &[
                "Make sure the Wi-Fi adapter is on: Action Center (bottom-right) → click the Wi-Fi tile to toggle it on; also check the physical Wi-Fi key (Fn+F-key) on laptops",
                "Check Airplane Mode is off: Settings → Network & Internet → Airplane mode → Off",
                "Restart WLAN AutoConfig service: PowerShell (admin) → Restart-Service Wlansvc -Force",
                "Toggle the adapter off and on: Device Manager → Network Adapters → right-click the Wi-Fi adapter → Disable device, wait 5 seconds, Enable device",
                "Update the Wi-Fi driver: Device Manager → Network Adapters → right-click the Wi-Fi adapter → Update driver → Search automatically",
                "If no Wi-Fi adapter appears in Device Manager: run hematite --inspect device_health and look for hidden or missing network devices; the adapter may need a driver install from the laptop/motherboard manufacturer",
                "For Intel Wi-Fi adapters: download the latest driver from Intel's website (search 'Intel Wi-Fi driver') — the Windows generic driver sometimes loses network scan capability after Windows updates",
                "Reset network settings as a last resort: PowerShell (admin) → netsh wlan delete profile name=* then restart; this removes all saved Wi-Fi profiles but often fixes a corrupt wireless profile store",
            ],
            dig_deeper: Some("wifi"),
        },
    },

    // ── Network share not accessible ─────────────────────────────────────────
    RecipeEntry {
        triggers: &[
            "server unreachable (ping failed)",
            "reachable:false",
            "share not accessible",
            "network path not found",
            "access is denied",
            "share access failed",
            "smb share unreachable",
            "network share",
        ],
        recipe: Recipe {
            severity: "INVESTIGATE",
            title: "Network share or mapped drive not accessible",
            steps: &[
                "Verify basic connectivity to the server: PowerShell → Test-NetConnection -ComputerName <server> -Port 445 (SMB port) — if this fails, the issue is network-level, not share-level",
                "Check the server is online: ping <server-name-or-IP> from PowerShell",
                "Confirm the share name is correct: Net View \\\\<server> lists all shares exposed by the server",
                "Re-enter credentials: Windows Credential Manager may have stale credentials — Win+R → control keymgr.dll → remove entries for the target server, then retry the share",
                "If 'Access Denied': verify your account has been granted share and NTFS permissions on the server side — contact the server admin to confirm your permissions",
                "Check SMB1 is not required: some old NAS devices only speak SMB1. Run hematite --inspect shares to see if SMB1 is disabled locally — if so, either enable it (security risk) or update the NAS firmware",
                "For domain environments: if the server name resolves but SMB fails, check if Kerberos is working — run hematite --inspect domain_health to verify DC reachability and GPO refresh",
                "Map the drive manually to force a fresh credential prompt: File Explorer → This PC → Map network drive → \\\\<server>\\<share> → check 'Connect using different credentials'",
            ],
            dig_deeper: Some("share_access"),
        },
    },

    // ── Microsoft Store / AppX not working ───────────────────────────────────
    RecipeEntry {
        triggers: &[
            "microsoft.windowsstore | status: missing",
            "appx |",
            "desktopappinstaller | status: missing",
            "store is not responding",
            "wsreset",
            "appx package",
            "microsoft store not opening",
            "microsoft store not working",
            "store app not installing",
        ],
        recipe: Recipe {
            severity: "ACTION",
            title: "Microsoft Store or AppX apps not working",
            steps: &[
                "Reset the Store cache: open Run (Win+R) → type wsreset.exe → Enter. A blank window opens, waits ~30 seconds, then the Store launches automatically. This clears the Store cache without removing app data.",
                "If the Store still won't open: PowerShell (admin) → Get-AppXPackage -AllUsers Microsoft.WindowsStore | Foreach {Add-AppxPackage -DisableDevelopmentMode -Register \"$($_.InstallLocation)\\AppXManifest.xml\"} to re-register the Store package",
                "Ensure the Windows Update service is running: PowerShell (admin) → Start-Service wuauserv — the Store update pipeline depends on Windows Update",
                "Check AppX installer services: run hematite --inspect installer_health to see if AppX or Store services are in a failed state",
                "For 'This app can't open' errors: Settings → Apps → Apps & features → find the app → Advanced options → Reset — this wipes the app's local data but usually fixes launch failures",
                "If apps fail to install from the Store with error code 0x80073D02 or 0x80073CF9: run: PowerShell (admin) → Get-AppXPackage -AllUsers | Foreach {Add-AppxPackage -DisableDevelopmentMode -Register \"$($_.InstallLocation)\\AppXManifest.xml\"} to rebuild the full AppX registration",
                "For persistent Store issues: Settings → System → Troubleshoot → Other troubleshooters → Windows Store Apps",
            ],
            dig_deeper: Some("installer_health"),
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
        match (
            self.action_count,
            self.investigate_count,
            self.monitor_count,
        ) {
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

    /// A plain-English sentence for non-technical users that explains what
    /// the grade means and what to do next.
    pub fn grade_intro(&self) -> &'static str {
        match self.grade {
            'A' => "Your PC is in great shape — no issues were found. The diagnostic data below is included for reference.",
            'B' => "Your PC is doing well, but there's one thing worth a closer look. The action plan below has specific steps.",
            'C' => "Your PC needs some attention. A couple of things should be investigated — follow the action plan below.",
            'D' => "Your PC needs attention. There are issues that should be fixed — follow the action plan below.",
            _ => "Your PC has critical issues that need immediate attention. Work through the action plan below as soon as possible.",
        }
    }
}

/// Compute a health grade (A–F) from diagnostic output sections.
pub fn score_health(outputs: &[(&str, &str)]) -> HealthScore {
    let all_recipes = collect_unique_recipes(outputs);

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
    let mut all_recipes = collect_unique_recipes(outputs);

    if all_recipes.is_empty() {
        return "No actionable findings — machine appears healthy.\n".to_string();
    }

    // Sort: ACTION first, then INVESTIGATE, then MONITOR
    all_recipes.sort_by_key(|r| match r.severity {
        "ACTION" => 0,
        "INVESTIGATE" => 1,
        _ => 2,
    });

    let mut out = String::with_capacity(all_recipes.len() * 200);
    for (i, recipe) in all_recipes.iter().enumerate() {
        let badge = match recipe.severity {
            "ACTION" => "⚠ ACTION REQUIRED",
            "INVESTIGATE" => "🔍 INVESTIGATE",
            _ => "📊 MONITOR",
        };
        let _ = write!(out, "### {}. {} — {}\n\n", i + 1, badge, recipe.title);
        for step in recipe.steps {
            out.push_str("- ");
            out.push_str(step);
            out.push('\n');
        }
        if let Some(_topic) = recipe.dig_deeper {
            out.push_str("\n*Run `hematite --diagnose` for a deeper automated investigation.*\n");
        }
        out.push('\n');
    }

    out
}

/// Format all matching recipes as an HTML fragment for embedding in a report page.
pub fn format_action_plan_html(outputs: &[(&str, &str)]) -> String {
    let mut all_recipes = collect_unique_recipes(outputs);

    if all_recipes.is_empty() {
        return "<p class=\"healthy\">No actionable findings — machine appears healthy.</p>\n"
            .to_string();
    }

    all_recipes.sort_by_key(|r| match r.severity {
        "ACTION" => 0,
        "INVESTIGATE" => 1,
        _ => 2,
    });

    let mut out = String::with_capacity(all_recipes.len() * 400);
    for (i, recipe) in all_recipes.iter().enumerate() {
        let (sev_class, badge_class, badge_text) = match recipe.severity {
            "ACTION" => ("sev-action", "b-action", "ACTION REQUIRED"),
            "INVESTIGATE" => ("sev-investigate", "b-investigate", "INVESTIGATE"),
            _ => ("sev-monitor", "b-monitor", "MONITOR"),
        };
        let _ = writeln!(out, "<div class=\"recipe {}\">", sev_class);
        let _ = writeln!(
            out,
            "<h3><span class=\"badge {}\">{}</span> {}. {}</h3>",
            badge_class,
            badge_text,
            i + 1,
            he(recipe.title)
        );
        out.push_str("<ol>\n");
        for step in recipe.steps {
            let _ = writeln!(out, "<li>{}</li>", he(step));
        }
        out.push_str("</ol>\n");
        if let Some(_topic) = recipe.dig_deeper {
            out.push_str(
                "<p class=\"dig-deeper\">Run <code>hematite --diagnose</code> for a deeper automated investigation of this issue.</p>\n"
            );
        }
        out.push_str("</div>\n");
    }
    out
}

use crate::agent::html_template::he;
