/// Cross-topic finding correlation — detects when independent inspect_host
/// findings share a common root cause and synthesizes a unified diagnosis.
///
/// Each rule requires ALL of its signals to appear in the combined raw output
/// (AND logic). When a rule fires it means two or more separate topics reported
/// conditions that almost certainly trace back to one underlying problem.

pub struct CorrelatedFinding {
    pub confidence: &'static str, // "HIGH" or "MEDIUM"
    pub summary: &'static str,    // one-sentence root cause
    pub detail: &'static str,     // why these findings are connected
    pub unified_steps: &'static [&'static str],
}

struct CorrelationRule {
    signals: &'static [&'static str], // ALL must appear in lowercased combined output
    confidence: &'static str,
    summary: &'static str,
    detail: &'static str,
    unified_steps: &'static [&'static str],
}

static RULES: &[CorrelationRule] = &[
    // ── Drive failure causing system crashes ──────────────────────────────────
    CorrelationRule {
        signals: &["healthstatus: unhealthy", "system crashes / unexpected shutdowns:"],
        confidence: "HIGH",
        summary: "Failing drive is almost certainly causing these system crashes",
        detail: "Disk health shows an unhealthy drive at the same time the crash log shows \
                 unexpected shutdowns. Drives with SMART failures cause read errors that \
                 Windows cannot recover from gracefully, triggering BSODs.",
        unified_steps: &[
            "Back up all important files immediately — do not wait, the drive may fail completely at any time",
            "Verify the failure: PowerShell (admin) → Get-PhysicalDisk | Select FriendlyName, HealthStatus — if Unhealthy, schedule replacement",
            "Run hematite --inspect disk_health for the full SMART status before replacing",
            "Replace the drive before doing anything else — no software fix will resolve hardware failure",
            "After replacement: reinstall Windows from USB or restore from backup",
        ],
    },

    // ── Drive failure + nearly full — critical double jeopardy ────────────────
    CorrelationRule {
        signals: &["healthstatus: unhealthy", "very low"],
        confidence: "HIGH",
        summary: "Drive is both failing and almost full — immediate data loss risk",
        detail: "A physically unhealthy drive that is also nearly out of space faces two \
                 simultaneous failure modes. Writes to a full drive with SMART errors have \
                 an elevated chance of corruption.",
        unified_steps: &[
            "URGENT: copy your most important files to an external drive or cloud storage right now",
            "Do not install anything, run updates, or make large writes until the drive is replaced",
            "Run hematite --inspect disk_health to confirm the SMART status",
            "Replace the drive as soon as possible — this combination is high-risk for total data loss",
        ],
    },

    // ── Drive saturation caused by SMART errors ───────────────────────────────
    CorrelationRule {
        signals: &["average disk queue", "healthstatus: unhealthy"],
        confidence: "HIGH",
        summary: "100% disk usage is caused by the failing drive retrying bad sectors",
        detail: "High disk queue length combined with an unhealthy SMART status is a classic \
                 pattern: the OS is retrying reads and writes on failing sectors, creating the \
                 appearance of disk saturation. The drive is the root cause, not software.",
        unified_steps: &[
            "Do not defrag or benchmark the drive — it will accelerate failure",
            "Back up data immediately, then replace the drive",
            "After replacement the disk saturation will resolve automatically",
            "Run hematite --inspect disk_health to get the exact SMART error count before replacing",
        ],
    },

    // ── Thermal throttling causing crashes ────────────────────────────────────
    CorrelationRule {
        signals: &["throttle reason:", "bsod (bugcheck)"],
        confidence: "HIGH",
        summary: "Overheating is triggering these system crashes",
        detail: "The overclocker telemetry shows active thermal throttling at the same time \
                 the crash log records BSODs. When a CPU or GPU exceeds its thermal limit the \
                 system can become unstable and crash rather than just slow down.",
        unified_steps: &[
            "Clean the CPU and GPU heatsinks with compressed air — dust buildup is the most common cause",
            "Check that all case fans are spinning and airflow is not blocked",
            "Run hematite --inspect thermal and --inspect overclocker to see current temps under load",
            "For laptops: use a cooling pad and ensure the bottom vents are not blocked",
            "If temps remain high after cleaning: reapply thermal paste (CPU) or replace thermal pads (GPU)",
            "Only investigate the BSODs further after the thermal issue is resolved — they may stop on their own",
        ],
    },

    // ── M365 auth broker cascade: Teams + Outlook both affected ──────────────
    CorrelationRule {
        signals: &[
            "tokenbroker | status: stopped",
            "classicteamscache |",
            "profilecount:",
        ],
        confidence: "HIGH",
        summary: "One auth broker failure is breaking Teams AND Outlook simultaneously",
        detail: "The Microsoft 365 token broker (WAM) is not running. Teams and Outlook both \
                 depend on the same authentication service — fixing the broker resolves sign-in \
                 failures in both apps at once. Clearing Teams cache or recreating Outlook \
                 profiles before fixing the broker will not work.",
        unified_steps: &[
            "Fix the auth broker first: PowerShell (admin) → Restart-Service TokenBroker -Force",
            "If the service won't start: PowerShell (admin) → sfc /scannow to repair the system files WAM depends on",
            "Sign out and back in to your work account: Settings → Accounts → Access work or school → disconnect → reconnect",
            "Only after the broker is running: clear Teams cache (%AppData%\\Microsoft\\Teams\\Cache, etc.)",
            "Only after the broker is running: test Outlook sign-in — in most cases it recovers without further steps",
            "If the device is Intune/AAD joined and the broker still fails: run dsregcmd /leave then dsregcmd /join (admin)",
        ],
    },

    // ── Auth broker + Teams only ──────────────────────────────────────────────
    CorrelationRule {
        signals: &["tokenbroker | status: stopped", "classicteamscache |"],
        confidence: "HIGH",
        summary: "Teams sign-in failure is caused by the M365 auth broker, not the cache",
        detail: "The token broker (WAM) is not running. Clearing the Teams cache without \
                 fixing the auth broker will result in Teams asking to sign in and then \
                 failing silently. Fix the broker first.",
        unified_steps: &[
            "Restart the token broker: PowerShell (admin) → Restart-Service TokenBroker -Force",
            "Verify it is running: Get-Service TokenBroker | Select Status",
            "Only after the broker is confirmed running: clear the Teams cache and relaunch Teams",
            "If sign-in still fails after clearing cache: run hematite --inspect identity_auth for the full auth chain state",
        ],
    },

    // ── Auth broker + Outlook only ────────────────────────────────────────────
    CorrelationRule {
        signals: &["tokenbroker | status: stopped", "profilecount:"],
        confidence: "HIGH",
        summary: "Outlook sign-in failure is caused by the M365 auth broker",
        detail: "The WAM token broker is not running. Outlook will loop on the sign-in screen \
                 or show repeated password prompts until the broker is restored. Recreating the \
                 mail profile before fixing the broker will reproduce the same failure.",
        unified_steps: &[
            "Restart the token broker: PowerShell (admin) → Restart-Service TokenBroker -Force",
            "Clear stale Office credentials: Win+R → control keymgr.dll → Windows Credentials → remove all MicrosoftOffice16_Data:SSPI:* entries",
            "Relaunch Outlook — it should sign in automatically once the broker is running",
            "If Outlook still fails: run hematite --inspect identity_auth for the full auth chain state",
        ],
    },

    // ── Pending reboot causing system crashes ─────────────────────────────────
    CorrelationRule {
        signals: &[
            "windows update requires a restart",
            "system crashes / unexpected shutdowns:",
        ],
        confidence: "HIGH",
        summary: "Incomplete Windows Update is likely causing these crashes",
        detail: "A Windows Update is staged but the system has not restarted to complete \
                 installation. Half-applied updates leave driver and system files in a mixed \
                 state that can cause BSODs and unexpected shutdowns until the restart completes.",
        unified_steps: &[
            "Save all open work and restart the computer to complete the pending update",
            "After restart, run Windows Update again to confirm no further updates are pending",
            "If crashes continue after the restart: run hematite --inspect recent_crashes to check for a different root cause",
        ],
    },

    // ── WMI corruption causing multiple failures ──────────────────────────────
    CorrelationRule {
        signals: &["wmi repository is inconsistent", "system crashes / unexpected shutdowns:"],
        confidence: "HIGH",
        summary: "WMI corruption is the likely root cause of multiple tool failures and crashes",
        detail: "WMI (Windows Management Instrumentation) is the data bus that Windows, \
                 Defender, PowerShell, and many system tools depend on. A corrupt repository \
                 causes cascading failures across diagnostics, updates, and system stability. \
                 Fix WMI before investigating any other issues.",
        unified_steps: &[
            "Stop WMI: PowerShell (admin) → net stop winmgmt /y",
            "Rebuild the repository: PowerShell (admin) → winmgmt /resetrepository",
            "Start WMI: PowerShell (admin) → net start winmgmt",
            "Verify: PowerShell → winmgmt /verifyrepository — should say 'WMI repository is consistent'",
            "Restart the machine after repair — WMI caches are session-scoped",
            "After restart, rerun hematite --triage to see if other issues have resolved themselves",
        ],
    },

    // ── VPN blocking internet connectivity ────────────────────────────────────
    CorrelationRule {
        signals: &["vpn adapter detected", "unreachable"],
        confidence: "MEDIUM",
        summary: "Active VPN may be causing the connectivity failure",
        detail: "A VPN adapter is present and active at the same time the connectivity \
                 check reports the internet as unreachable. VPN misrouting, split-tunnel \
                 issues, or a disconnected VPN tunnel that still holds the routing table \
                 are common causes of this pattern.",
        unified_steps: &[
            "Disconnect the VPN and test internet connectivity directly: ping 1.1.1.1",
            "If internet works without VPN: the VPN client or tunnel config is the problem — contact your VPN provider or IT admin",
            "If internet is still broken without VPN: run hematite --inspect connectivity for the base network issue",
            "Flush DNS after disconnecting VPN: PowerShell (admin) → ipconfig /flushdns",
            "Check for stale routes left by the VPN: PowerShell (admin) → route print — delete any routes pointing to the VPN adapter",
        ],
    },

    // ── Teams cache + crash evidence ─────────────────────────────────────────
    CorrelationRule {
        signals: &["classicteamscache |", "application error |"],
        confidence: "MEDIUM",
        summary: "Bloated Teams cache is likely causing Teams to crash",
        detail: "The Teams cache directory is large and there is crash evidence in the \
                 Application event log. A corrupted or oversized cache is the #1 cause of \
                 Teams instability — clearing it resolves most Teams crash and hang issues.",
        unified_steps: &[
            "Quit Teams completely: right-click the system tray icon → Quit",
            "Clear Classic Teams cache: Win+R → %AppData%\\Microsoft\\Teams → delete Cache, blob_storage, databases, GPUCache, IndexedDB, Local Storage, tmp",
            "Clear New Teams cache: Win+R → %LocalAppData%\\Packages\\MSTeams_8wekyb3d8bbwe\\LocalCache\\ → delete all contents",
            "Relaunch Teams and sign in — the cache rebuilds from the server automatically",
            "If Teams still crashes after cache clear: run hematite --inspect identity_auth to check auth broker state",
        ],
    },

    // ── Defender disabled with active outbound connections ───────────────────
    CorrelationRule {
        signals: &["real-time protection: off", "established"],
        confidence: "MEDIUM",
        summary: "Defender is disabled while the machine has active network connections — security risk",
        detail: "Real-time protection is off at the same time the machine has established \
                 TCP connections. Without Defender active, malware can exfiltrate data or \
                 phone home over these connections without being intercepted.",
        unified_steps: &[
            "Re-enable Defender real-time protection immediately: Windows Security → Virus & threat protection → Real-time protection → On",
            "Run a full scan: Windows Security → Virus & threat protection → Quick scan (then Full scan if quick finds anything)",
            "Check what process owns the active connections: run hematite --inspect connections and look for unknown remote addresses",
            "If you cannot turn Defender back on, a third-party AV or Group Policy may be blocking it — run hematite --inspect security for the full picture",
        ],
    },

    // ── Thermal throttling causing high CPU readings (no crash) ──────────────
    CorrelationRule {
        signals: &["[warning] cpu load is extremely high", "throttle reason:"],
        confidence: "HIGH",
        summary: "Thermal throttling is the root cause of the high CPU readings",
        detail: "The system reports near-100% CPU at the same time the thermal subsystem \
                 is actively throttling the processor. Windows measures CPU utilization as \
                 a fraction of the current throttled frequency — a CPU running at 30% of \
                 its rated speed will show 100% load even under a light workload. Fixing \
                 the thermal issue will resolve the apparent CPU saturation without touching \
                 any running applications.",
        unified_steps: &[
            "Clean the CPU and GPU heatsinks with compressed air — dust buildup is the most common cause of thermal throttling",
            "Check that all case fans are spinning and airflow paths are not blocked",
            "For laptops: run on a hard flat surface with bottom vents unobstructed, or use a cooling pad",
            "Run hematite --inspect thermal to see current temperatures under load",
            "Run hematite --inspect overclocker to see the active throttle reason (Power vs Thermal)",
            "If temperatures are normal but throttling persists: check the active power plan — Power Saver or Balanced can cap CPU frequency",
            "After fixing cooling: the high CPU reading will drop because the chip can now run at its rated speed",
        ],
    },

    // ── RAM pressure causing disk saturation via pagefile swapping ───────────
    CorrelationRule {
        signals: &["[warning] memory usage is near capacity", "average disk queue"],
        confidence: "HIGH",
        summary: "Low RAM is causing disk saturation through pagefile swapping",
        detail: "Resource monitor shows RAM near capacity at the same time disk queue \
                 length is elevated. When physical RAM is exhausted, Windows offloads \
                 active memory pages to the pagefile on the system drive. This appears \
                 as near-100% disk usage and makes the machine feel completely \
                 unresponsive. The root cause is RAM pressure, not a disk hardware problem.",
        unified_steps: &[
            "Identify the RAM consumer: Task Manager → Performance → Memory → Open Resource Monitor → Memory tab",
            "Close unused apps — browsers (50–200 MB per tab), Electron apps (Teams, Slack, VS Code), and IDEs are the most common culprits",
            "Run hematite --inspect resource_load to rank processes by memory consumption right now",
            "If RAM is chronically full with normal use: this machine needs a RAM upgrade",
            "Short-term relief: increase the pagefile size — System Properties → Advanced → Performance Settings → Advanced → Virtual Memory → Custom size",
            "Do not defrag while this condition is active — the disk is already under heavy pagefile I/O load",
        ],
    },

    // ── Windows Installer disabled + CBS reboot pending ──────────────────────
    CorrelationRule {
        signals: &[
            "windows installer service (msiserver) is disabled",
            "windows component install/update requires a restart",
        ],
        confidence: "HIGH",
        summary: "Pending reboot and disabled Windows Installer are both blocking software installations",
        detail: "The Windows Installer service (msiserver) is disabled — no MSI-based \
                 application can install or update. A Windows component update is also \
                 staged and waiting for a restart. The pending reboot may restore the \
                 Installer service as part of CBS completion, so restart first before \
                 manually re-enabling.",
        unified_steps: &[
            "Restart the computer first — this completes the pending Windows component update (CBS)",
            "After restart: check if Windows Installer recovered: PowerShell → Get-Service msiserver",
            "If msiserver is still disabled: PowerShell (admin) → Set-Service msiserver -StartupType Manual; Start-Service msiserver",
            "Verify: Get-Service msiserver | Select Status, StartType — expect Running / Manual",
            "Retry the failed installation once both fixes are confirmed",
            "Run hematite --inspect installer_health after restart to verify all install-related services are healthy",
        ],
    },
];

/// Run all correlation rules against the combined raw inspect output.
/// Returns all rules where every signal is present (AND logic).
/// Results are ordered HIGH confidence first.
pub fn correlate_findings(raw_output: &str) -> Vec<CorrelatedFinding> {
    let lower = raw_output.to_ascii_lowercase();
    let mut results: Vec<CorrelatedFinding> = RULES
        .iter()
        .filter(|rule| rule.signals.iter().all(|s| lower.contains(*s)))
        .map(|rule| CorrelatedFinding {
            confidence: rule.confidence,
            summary: rule.summary,
            detail: rule.detail,
            unified_steps: rule.unified_steps,
        })
        .collect();

    // HIGH confidence first, MEDIUM second.
    results.sort_by_key(|r| if r.confidence == "HIGH" { 0u8 } else { 1 });
    results
}
