# Hematite CLI Documentation

## What this project is

Hematite is a local AI coding harness and natural-language Senior SysAdmin and Network Admin assistant built in Rust. It runs on your machine and uses any OpenAI-compatible local model server. The default target is LM Studio on `localhost:1234`, but the endpoint is configurable. The terminal TUI is one interface layer of the product, not the whole product. The main engineering target is a single-GPU consumer Windows setup, especially RTX 4070-class hardware.
It features a high-fidelity integrated host inspection suite covering **116+ read-only diagnostic topics** for precision triage.

Hematite supports two model protocol paths:

- **Gemma 4 native** — Gemma 4 family models; native tool markup auto-enabled by model name (`gemma_native_auto: true` by default)
- **Standard OpenAI-compatible** — all other models; plain tool format; tested primary target is Qwen/Qwen3.5-9B Q4_K_M
- **Ollama** — supported as a local runtime when `api_url` points at an Ollama-compatible endpoint such as `http://localhost:11434/v1`

## Build and Run

```powershell
cargo build
cargo run
cargo run -- --no-splash
cargo run -- --rusty
cargo run -- --yolo
cargo run -- --brief
cargo run -- --stats
cargo run -- --teleported-from <path>
pwsh ./clean.ps1
```

## Core Protocol: Teleportation & Handshake

- **Workspace Teleportation**: When diving into a new directory, Hematite spawns a fresh terminal session pre-navigated to the target. The new window opens at the same pixel size and position as the originating window, and launches with `--no-splash` for a seamless transition.
- **Self-Destruct**: The original terminal session performs a clean exit after the handoff to ensure workstation hygiene. A background watcher detects when Hematite exits and kills the originating `cmd.exe`. Windows Terminal tabs are explicitly excluded (killing `WindowsTerminal.exe` would close all tabs).
- **Teleportation Handshake**: New sessions arriving via teleportation (flagged by `--teleported-from`) display a specialized greeting confirming the origin and intent.
- **OS Shortcut Directory Guard**: Teleporting to or launching from Desktop, Downloads, Documents, Pictures, Videos, or Music does not create a local `.hematite/` folder there. All runtime state (settings, vein, session, logs, scratch) routes to `~/.hematite/` instead, keeping OS directories clean. Real project directories are unaffected.
- **Local Search Stack**: When `auto_start_searx` is enabled, Hematite scaffolds and boots a private SearXNG stack under `~/.hematite/searxng-local` when the configured `searx_url` is local and Docker Desktop is available. If SearXNG is already reachable, Hematite reuses it instead of restarting it. If Docker is missing or the daemon is stopped, Hematite surfaces a compact startup note with the fix instead of silently failing. Set `HEMATITE_SEARX_ROOT` to relocate the stack; `auto_stop_searx` only stops the instance Hematite started in the current session. The default scaffold now favors a safer technical-source engine pool instead of the older broad 12-engine mix.

> **Important:** `cargo build` / `cargo run` only update `target/debug/hematite.exe`. If you run
> Hematite from the portable dist (`dist\windows\Hematite-X.Y.Z-portable\hematite.exe`) — which is
> what end-users have on their PATH — you must rebuild the portable bundle after any code change:
>
> ```powershell
> pwsh ./scripts/package-windows.ps1
> ```
>
> `cargo run` is the fastest loop during development. Run the package script before testing with
> the portable binary or before committing/tagging a release.
>
> **Agent rule:** if you are operating this repo through an external harness with sandboxed tools,
> do not start local Windows build/package/install steps in the sandbox. `cargo build --release`,
> `pwsh ./scripts/package-windows.ps1`, installer generation, and `-AddToPath` can touch the local
> ORT cache in `AppData`, release sidecars, `dist/`, and the real user `PATH`. Treat those as
> unrestricted local-machine operations first; use sandboxed runs for source inspection,
> read-only analysis, and isolated code execution.

## Hotkeys and Commands

- `ESC`: cancel the current task and copy the session transcript to the clipboard
- `Ctrl+Q` / `Ctrl+C`: exit Hematite and copy the session transcript
- `Ctrl+T`: toggle voice
- `Ctrl+O`: open file picker to attach a document (PDF/markdown/txt) for the next turn
- `Ctrl+I`: open file picker to attach an image for the next turn (vision path)
- `Ctrl+Z`: undo last file edit (ghost backup restore)
- `@` in input: opens live file autocomplete — scans workspace, filters as you type, optimized with **Smart Splicing** for Path Aliases (e.g. `@DESKTOP`); Tab/Enter/Mouse-click inserts the path
- `/read <text>`: speaks text aloud directly through the TTS engine, bypassing the model — ESC stops playback
- `Y` / `N`: approve or skip a diff preview modal when the model proposes an edit
- `/voice`: list all available TTS voices with numbers
- `/voice N` or `/voice <id>`: select a voice by number or ID — saves to `.hematite/settings.json` and takes effect immediately
- `/attach <path>`: attach a PDF, markdown, or text file as context for the next message then clear
- `/image <path>`: attach an image for the next message — passed to the model via the vision path
- `/detach`: drop any pending document or image attachment without sending
- `/copy`: copy the session transcript manually
- `/clear`: clear visible dialogue and side-panel session state
- `/forget`: purge saved conversation memory and wipe visible session state
- `/new`: reset session history while keeping project memory
- `/cd <path>`: teleport to any directory — opens a fresh Hematite session there and closes this one. Supports bare tokens like `downloads`, `desktop`, `docs`, `pictures`, `videos`, `music`, `home`, `temp`, bare `~`, `@TOKENS`, `..`, and absolute paths.
- `/ls`: show a numbered navigation map — common OS locations + subdirectories of the current directory. Type `/ls <N>` to teleport directly to entry N. `/ls <path>` lists subdirectories of any path.
- `/ask [prompt]`: sticky read-only analysis mode
- `/code [prompt]`: sticky implementation mode
- `/architect [prompt]`: sticky plan-first mode that persists a reusable handoff
- `/implement-plan`: execute the saved architect handoff in `/code`
- `/read-only [prompt]`: sticky hard no-mutation mode
- `/teach [prompt]`: sticky teacher mode — inspects real machine state first, then delivers a grounded numbered walkthrough for any admin/config/system task; does not execute write operations itself
- `/auto`: return to the narrowest-effective workflow mode
- `/chat [prompt]`: sticky conversational mode — lighter prompt, no coding scaffolding, tools still available
- `/agent`: alias for returning to the full coding agent mode from chat
- `/think [prompt]`: enable extended reasoning (thinking tokens) for the next turn or sticky
- `/no_think [prompt]`: disable thinking tokens — faster, lower token cost
- `/gemma-native`: toggle Gemma 4 native tool markup on/off manually (auto-detected by default)
- `/swarm`: trigger parallel worker agents
- `/worktree [branch]`: create or switch to a git worktree for isolated branch work
- `/lsp`: show LSP server status and active language server diagnostics
- `/reroll`: hatch a new companion soul mid-session (soul/personality reroll)
- `/rules [view|edit]`: view status, inspect content, or edit project guidelines (.hematite/rules.md)
- `/runtime-refresh`: force a resync of the LM Studio model profile and context window size
- `/vein-inspect`: inspect indexed Vein memory, hot files, and active room bias
- `/vein-reset`: wipe the Vein index and rebuild from scratch on the next turn
- `/workspace-profile`: inspect the auto-generated workspace profile
- `/rules`: show which behavioral rule files exist and their load status
- `/rules view`: display combined content of all active rule files (CLAUDE.md, .hematite/rules.md, etc.)
- `/rules edit`: open `.hematite/rules.local.md` in the system editor (private, gitignored)
- `/rules edit shared`: open `.hematite/rules.md` in the system editor (shared, committed with the repo)
- Rule files are injected into the system prompt every turn automatically — no restart needed after editing
- `.hematite/instructions/`: drop any `<topic>.md` file here for topic-scoped rules that only inject when the turn context mentions that topic name (e.g. `authentication.md` injects only when the user's message references authentication)
- `/diff`: show a diff of the last file edit made this session
- `/undo`: undo the last file edit by restoring from the ghost backup
- `/health`: run a quick workstation health check via `inspect_host(topic: "health_report")`
- `/explain [prompt]`: explain the current file or selection in plain English
- `/version`: show the running Hematite release version plus build state
- `/about`: show author, repo, and product info
- `hematite --version`: print the same build report from the CLI
- `/copy`: copy the session transcript manually
- `/copy-clean`: copy the transcript with tool calls stripped — prose only
- `/copy-last`: copy only the last assistant response
- `/clear`: clear visible dialogue and side-panel session state
- `/cd <path>`: teleport to any directory — opens a fresh Hematite session there and closes this one. Supports bare tokens like `downloads`, `desktop`, `docs`, `pictures`, `videos`, `music`, `home`, `temp`, bare `~`, `@TOKENS`, `..`, and absolute paths.
- `/attach <path>`: attach a PDF, markdown, or text file as context for the next message then clear
- `/attach-pick`: open a file picker to select a document attachment
- `/image <path>`: attach an image for the next message via the vision path
- `/image-pick`: open a file picker to select an image attachment
- `/detach`: drop any pending document or image attachment without sending

Requires LM Studio running locally with a model loaded and the server started on port `1234`.

Practical rule: the version/build label is compile-time metadata. A new commit or tag does not change what the already-built binary reports. Rebuild the binary or rerun `pwsh ./scripts/package-windows.ps1 -AddToPath` if you want `hematite --version`, `/version`, and the startup banner to reflect the latest commit, tag, or dirty/clean state.

Attribution rule: authorship and identity questions should resolve from Hematite's fixed app metadata, not from model improvisation. `/about` is the operator-facing path, and prompts like `who created you` or `who engineered Hematite` should answer with Ocean Bennett as the creator and maintainer.

Package naming rule: the crates.io package is `hematite-cli`, but the executable name stays `hematite`. Keep that split so the package namespace is distinct while the operator command stays short.

Crates.io publish order: publish `hematite-kokoros` first, then publish `hematite-cli`. The main package depends on the forked voice crate by published package name while keeping the source-level crate path as `kokoros`.

Crates.io compatibility rule: the default published/source build does not embed the large Kokoro voice assets. Packaged releases and local packaging scripts must build with `--features embedded-voice-assets` so the shipped Windows/macOS/Linux bundles keep the baked-in voice engine.

Crates.io update rule: in normal use, almost every public tagged Hematite release should republish `hematite-cli`. Republish `hematite-kokoros` only when the vendored fork itself changes. Do not bump the voice fork just because the main app shipped a new release.

- **Host Inspection Priority**: For all diagnostic questions (load, network, processes, log-checks), the agent **MUST** prefer `inspect_host` over raw `shell`.
- **Native Diagnostic Lane**: For all hardware intensity, throughput, or disk benchmarking tasks, the agent **MUST** use `inspect_host(topic: "disk_benchmark")`. 
- **Auto-Fallback Robustness**: If a requested target binary (e.g. `.hematite/hematite.exe`) is not found, the `disk_benchmark` tool will automatically pivot to benchmarking the current running executable. Do not fail if a path is missing—run the benchmark on the current binary to provide data.
- **Redirection Discipline**: Common diagnostic commands and read-only metadata checks (e.g. `arp -a`, `Get-Process`, `Get-Item`, `Test-Path`, `Select-Object`) are silently redirected or whitelisted. These are safe operations.
- **Telemetry Whitelisting**: `get-counter`, `Get-Item`, `Test-Path`, and `Select-Object` are whitelisted in `guard.rs` to reduce operator approval friction during system audits.
- **Harness Pre-Run**: When the user asks about 2+ inspection topics in one message, Hematite executes all queries *before* the model turn begins. Results are injected into the **conversation history** as simulated turns (assistant calls + tool results). This ensures the model "sees" the data in the official transcript, which prevents redundant tool calls or orchestration loops.
- **Multi-Topic Rule**: Never collapse multiple distinct topics into a single generic topic like `"network"`. Each topic must be called separately. Example: "show route table, ARP, DNS cache, and traceroute" → four separate inspect_host calls (or one harness pre-run covering all four).
- **Storage Inspection**: Use `topic: "storage"` for all-drives capacity with ASCII bar charts, developer cache directory sizing, and **Real-time Disk Intensity** (`Average Disk Queue Length`).
- **Hardware Inventory**: Use `topic: "hardware"` for full hardware DNA — CPU model/cores/clock, RAM total/speed/sticks, GPU name/driver/resolution, motherboard/BIOS manufacturer/version, and **Virtualization Health** (Hypervisor status and SLAT/VT-x capability).
- **Session Audit**: Use `topic: "sessions"` for active and disconnected user logon sessions.
- **Health Report**: Use `topic: "health_report"` (or alias `"system_health"`) for a tiered plain-English verdict (ALL GOOD / WORTH A LOOK / ACTION REQUIRED) across disk, RAM, tools, and recent error events.
- **Windows Update**: Use `topic: "updates"` for last install date, pending update count, and Windows Update service state.
- **Security Status**: Use `topic: "security"` for Defender real-time protection, last scan age, signature freshness, firewall profile states, Windows activation, and UAC state.
- **Pending Reboot**: Use `topic: "pending_reboot"` to check if a restart is queued (Windows Update, CBS, file rename operations) and why.
- **Drive SMART Health**: Use `topic: "disk_health"` for physical drive health via Get-PhysicalDisk and SMART failure prediction.
- **Battery**: Use `topic: "battery"` for charge level, status, estimated runtime, and wear level — reports no battery gracefully on desktops.
- **Crash History**: Use `topic: "recent_crashes"` for BSOD/unexpected shutdown events and application crash/hang events from the Windows event log.
- **Application Crashes**: Use `topic: "app_crashes"` for detailed application crash and hang triage — faulting application name, version, faulting module, exception code, crash frequency, WER archive count. Accepts optional `process` arg to filter by app name (e.g. `process: "chrome.exe"`). Use `recent_crashes` for BSOD/kernel panics instead.
- **Device Health**: Use `topic: "device_health"` for precision detection of malfunctioning hardware (PnP "Yellow Bangs") via ConfigManager error codes.
- **Drivers**: Use `topic: "drivers"` for a comprehensive audit of active system drivers and their operational states.
- **Peripherals**: Use `topic: "peripherals"` for a deep-dive into USB controllers, HID devices (Keyboard/Mouse), and connected monitors.
- **Scheduled Tasks**: Use `topic: "scheduled_tasks"` for all non-disabled scheduled tasks with name, path, last run time, and executable.
- **Thermal Health**: Use `topic: "thermal"` for real-time telemetry — CPU temp, thermal margins, ACPI fallback sensing, and active throttling indicators.
- **Windows Activation**: Use `topic: "activation"` for license state, genuine status, and Product ID/Key metadata.
- **Patch History**: Use `topic: "patch_history"` for Windows HotFix and KB update audit (last 48h focus).
- **Repo Doctor**: Use `topic: "repo_doctor"` to inspect workspace health — git status, uncommitted changes, and build-file presence.
- **Disk Benchmark**: Use `topic: "disk_benchmark"` for sequential read/write throughput and latency measurements.
- **Dev Conflicts**: Use `topic: "dev_conflicts"` for cross-tool environment conflict detection — Node.js version managers, Python 2/3 ambiguity, conda shadowing, Rust toolchain path conflicts, Git identity/signing, and duplicate PATH entries.
- **Connectivity Check**: Use `topic: "connectivity"` to test internet reachability and DNS resolution — reports REACHABLE/UNREACHABLE with DNS pass/fail and gateway/VPN context.
- **Wi-Fi Status**: Use `topic: "wifi"` for wireless adapter state, SSID, signal strength (RSSI/quality), band, channel, and negotiated speed.
- **Active TCP Connections**: Use `topic: "connections"` for active/established TCP connections — remote address, port, process name, and connection state.
- **VPN Status**: Use `topic: "vpn"` to detect active VPN adapters, tunnel IPs, and any recognized VPN client services.
- **Proxy Settings**: Use `topic: "proxy"` for WinHTTP system proxy, Internet Options proxy, and environment variable proxy config.
- **Firewall Rules**: Use `topic: "firewall_rules"` for non-default enabled Windows Firewall rules — direction, action (Allow/Block), and profile.
- **BitLocker**: Use `topic: "bitlocker"` for drive encryption status — BitLocker volume status, protection state (ON/OFF), and encryption percentage.
- **RDP Status**: Use `topic: "rdp"` for Remote Desktop configuration — enabled state, port (default 3389), NLA requirements, firewall rule check, and active RDP sessions.
- **Shadow Copies**: Use `topic: "shadow_copies"` for Volume Shadow Copies (VSS) — lists snapshot history, storage allocation, and recent system restore points.
- **Page File**: Use `topic: "pagefile"` for virtual memory configuration — page file paths, allocated size, current usage, and peak usage relative to RAM.
- **Windows Features**: Use `topic: "windows_features"` for enabled Windows optional features — lists all ON features and flags notable ones (IIS, Hyper-V, WSL).
- **Printers**: Use `topic: "printers"` for installed printers — printer name, driver, port, status, and active print job queue.
- **WinRM**: Use `topic: "winrm"` for Windows Remote Management — service state, listeners, PS Remoting status (WS-Man test), and TrustedHosts list.
- **Network Stats**: Use `topic: "network_stats"` for adapter-level throughput analytics — TX/RX bytes (MB), packet errors, and discarded/dropped packets since boot.
- **UDP Listeners**: Use `topic: "udp_ports"` for active UDP listeners — local address, port, PID, process name, and annotations for well-known ports (DNS, DHCP, NTP).
- **Group Policy (GPO)**: Use `topic: "gpo"` for applied Group Policy Objects — shows applied computer-scope GPOs and filtering status. Requires Administrator elevation on Windows.
- **Certificates**: Use `topic: "certificates"` for local personal certificates — lists subjects, thumbprints, and expiry dates (flags those expiring within 30 days).
- **Integrity**: Use `topic: "integrity"` for Windows component store health — checks SFC/DISM status (Corrupt/AutoRepairNeeded) via registry and log visibility.
- **Share Access**: Use `topic: "share_access"` for readability and connectivity testing for specific network shares and UNC paths.
- **Directory Audit**: Use `topic: "directory"`, `"desktop"`, or `"downloads"` for directory listing and file metadata.
- **Traceroute**: Use `topic: "traceroute"` to trace the network path to a host (default 8.8.8.8). Accepts optional `host` arg. Uses `tracert` on Windows, `traceroute`/`tracepath` on Linux/macOS.
- **DNS Cache**: Use `topic: "dns_cache"` to inspect locally cached DNS entries — hostname, record type, resolved address, and TTL.
- **ARP Table**: Use `topic: "arp"` for the ARP neighbor table — IP-to-MAC mappings for devices on the local network.
- **Route Table**: Use `topic: "route_table"` for the system routing table — destination prefixes, next hops, metrics, and interface names.
- **Heuristic Command Sanitizer**: Hematite enforces a hard gate that blocks tool calls containing natural language sentences in command arguments. Never pass conversational "overthinking" into shell tools; use surgical, machine-readable commands only.
- **Proactive Research Priority**: When answering technical questions about API versions, library changes, or news since 2024, the agent **MUST** prefer `research_web` over internal knowledge. Verifying technical uncertainty is a core behavioral requirement.
- **Environment Variables**: Use `topic: "env"` to inspect environment variables — shows developer/tool vars (CARGO_HOME, JAVA_HOME, GOPATH, etc.) and redacts secret-shaped values (KEY, TOKEN, PASSWORD) to presence-only.
- **Hosts File**: Use `topic: "hosts_file"` to read `/etc/hosts` (Windows: `drivers\etc\hosts`) — active entries, custom non-loopback entries flagged, full file content shown.
- **Docker**: Use `topic: "docker"` for Docker daemon state, running containers, local images, Compose projects, and active context. Reports gracefully if Docker is not installed or daemon is not running.
- **Docker Filesystems**: Use `topic: "docker_filesystems"` for bind mounts, named volumes, per-container mount summaries, and Docker Desktop disk-image growth. Output is shaped as `finding -> impact -> fix`.
- **WSL**: Use `topic: "wsl"` for Windows Subsystem for Linux — installed distros, running state, WSL version. Windows-only; reports gracefully on Linux/macOS.
- **WSL Filesystems**: Use `topic: "wsl_filesystems"` for WSL rootfs usage, host-side `ext4.vhdx` growth, and `/mnt/c` bridge health without starting stopped distros.
- **LAN Discovery**: Use `topic: "lan_discovery"` for neighborhood, NAS/printer visibility, NetBIOS/SMB browse evidence, mDNS/SSDP/UPnP listener surface, gateway/device-discovery hints, and a plain-English diagnosis path for discovery failures.
- **Audio**: Use `topic: "audio"` for Windows Audio service health, playback and recording endpoint inventory, speaker and microphone path triage, and Bluetooth-audio crossover.
- **Bluetooth**: Use `topic: "bluetooth"` for radio presence, Bluetooth service health, paired-device inventory, reconnect and pairing issues, and headset-role diagnostics.
- **Camera**: Use `topic: "camera"` for PnP camera/webcam device inventory, Windows camera privacy registry state, Windows Hello biometric camera detection, and plain-English diagnosis for "camera not working / blocked by privacy settings".
- **Sign-In / Windows Hello**: Use `topic: "sign_in"` for Windows Hello and biometric service state (WBioSrvc), recent logon failure events (EventID 4625), enrolled credential providers, and plain-English diagnosis for "PIN/fingerprint not working / can't sign in".
- **Installer Health**: Use `topic: "installer_health"` for Windows Installer (`msiserver`), AppX/Store install services, `winget`/Desktop App Installer presence, Microsoft Store package health, reboot or in-progress installer blockers, and recent MSI/AppX failure evidence.
- **OneDrive**: Use `topic: "onedrive"` for client install/running state, configured accounts, sync-root existence, policy blockers, and Known Folder Backup/Desktop/Documents/Pictures redirection state.
- **Browser Health**: Use `topic: "browser_health"` for Edge/Chrome/Firefox inventory, default browser and protocol associations, WebView2 runtime health, browser proxy/policy overrides, profile/cache pressure, and recent browser crash evidence.
- **Identity Auth**: Use `topic: "identity_auth"` for Microsoft 365 token-broker and Web Account Manager health, AAD Broker Plugin presence, `dsregcmd` device registration state, Office/Teams/OneDrive account mismatch clues, WebView2 auth dependency state, and recent auth-related events.
- **Outlook**: Use `topic: "outlook"` for classic Outlook and new Outlook for Windows install inventory, running process state and RAM usage, mail profile count, OST and PST file discovery with sizes, add-in inventory with load behavior and resiliency-disabled items, authentication and token broker cache state, and recent Outlook crash evidence from the Application event log.
- **Teams**: Use `topic: "teams"` for classic Teams and new Teams (MSTeams MSIX) install inventory, running process state and RAM usage, cache directory sizing (the #1 Teams fix), WebView2 runtime dependency check, account and sign-in state from registry, audio/video device binding from config files, and recent Teams crash evidence from the Application event log.
- **Windows Backup**: Use `topic: "windows_backup"` for File History service state and last backup date/target drive, Windows Backup (wbadmin) last successful backup and scheduled tasks, System Restore enabled state and most recent restore point, OneDrive Known Folder Move per-account protection state, and recent backup failure events from the Application event log.
- **Event Query**: Use `topic: "event_query"` for targeted Windows Event Log filtering by Event ID, source/provider, log name, severity level, and time window. Supports prompts like "System errors in the last 4 hours", Event ID 4625 failed-logon review, 7034 service crash search, and 41 unexpected-shutdown triage.
- **Search Index**: Use `topic: "search_index"` for Windows Search (WSearch) service state, indexer registry configuration, indexed locations, recent indexer errors, and plain-English diagnosis for "search not finding files / indexer stopped".
- **Display Config**: Use `topic: "display_config"` for active monitor resolution, refresh rate, DPI/scaling, video adapter driver version, and connected monitor names — answers "what refresh rate / how many monitors / is my DPI correct".
- **NTP / Time Sync**: Use `topic: "ntp"` for Windows Time service (W32Time) health, NTP source and last sync via w32tm, configured NTP peers, and plain-English diagnosis for clock drift or sync failure.
- **CPU Power**: Use `topic: "cpu_power"` for active power plan, processor min/max state, turbo boost mode, current CPU clock and load, thermal zone temperatures, and diagnosis for "CPU stuck slow / boost disabled / power plan capping frequency".
- **Credentials**: Use `topic: "credentials"` for Windows Credential Manager vault summary, credential target inventory, type counts, and hygiene warnings without ever exposing secret values.
- **TPM / Secure Boot**: Use `topic: "tpm"` for TPM presence/readiness/spec version, Secure Boot state, firmware mode (UEFI vs legacy BIOS), and plain-English diagnosis for Windows 11 or BitLocker security posture.
- **SSH**: Use `topic: "ssh"` for SSH client version, sshd service state, `~/.ssh` directory inventory (known_hosts count, authorized_keys count, private key files), and `~/.ssh/config` host entries.
- **Installed Software**: Use `topic: "installed_software"` for installed programs — winget list on Windows (registry fallback), dpkg/rpm/pacman on Linux, brew + mas on macOS.
- **Git Config**: Use `topic: "git_config"` for global git configuration audit — user identity, core settings, signing config, push/pull defaults, credential helper, branch defaults, local repo config, and git aliases.
- **Databases**: Use `topic: "databases"` to detect running local database engines — PostgreSQL, MySQL/MariaDB, MongoDB, Redis, SQLite, SQL Server, CouchDB, Cassandra, Elasticsearch — via CLI version check, TCP port probe, and OS service state. No credentials required.
- **Overclocker Telemetry**: Use `topic: "overclocker"` for precision silicon performance — NVIDIA clocks, fans, board power and power-cap context (W), explicit GPU-voltage availability reporting, firmware-reported CPU voltage when WMI exposes it, root-cause throttle decoding (Power vs Thermal), and session history trends (Temp/Clock drift anomalies).
- **Hyper-V**: Use `topic: "hyperv"` for Hyper-V role state (VMMS service, feature installed), VM inventory (name, state, CPU%, RAM, uptime), VM network switches (External/Internal/Private with bound NIC), VM checkpoint listing (flags excessive checkpoints), and host RAM overcommit detection. Reports gracefully if Hyper-V is not installed.
- **User Accounts**: Use `topic: "user_accounts"` for local user accounts (name, enabled state, last logon, password required), Administrators group members, active logon sessions, and whether the current process is running elevated.
- **Audit Policy**: Use `topic: "audit_policy"` for Windows audit policy (auditpol /get /category:*) — shows which event categories are logging Success/Failure. Flags if no categories are enabled. Requires Administrator elevation on Windows; falls back to auditd on Linux.
- **Shares**: Use `topic: "shares"` for SMB shares this machine is exposing (flags custom non-admin shares), SMB server security settings (SMB1/SMB2 state, signing required, encryption), and mapped network drives. Warns if SMB1 is enabled.
- **DNS Servers**: Use `topic: "dns_servers"` for the DNS resolvers configured per network adapter (not cache — the actual configured nameservers), annotated with well-known providers (Google, Cloudflare, Quad9, OpenDNS), DoH configuration, and DNS search suffix list.
- **Latency**: Use `topic: "latency"` for ping RTT (min/avg/max) and packet loss to the default gateway, Cloudflare DNS (1.1.1.1), and Google DNS (8.8.8.8) — findings for unreachable targets, high packet loss (≥25%), and elevated average RTT (>150ms).
- **Network Adapter**: Use `topic: "network_adapter"` for NIC inventory (link speed, MAC, driver version), offload settings (LSO/RSS/TCP checksum offload/jumbo frames) per adapter, error and discard counters, and wake-on-LAN / power management state; findings for adapter errors and half-duplex mismatches.
- **DHCP Lease**: Use `topic: "dhcp"` for DHCP lease details per adapter — server IP, lease obtained time, lease expires time, subnet mask, DNS servers assigned by DHCP; findings for expired or imminently-expiring leases.
- **MTU**: Use `topic: "mtu"` for per-adapter IPv4/IPv6 MTU and path MTU discovery (DF-bit ping test to 8.8.8.8 at 1472/1400/1280/576 bytes); findings for restricted MTU, VPN fragmentation issues, or ICMP-blocked paths.
- **IPv6**: Use `topic: "ipv6"` for per-adapter IPv6 addresses (global/link-local/ULA) with prefix origin (SLAAC/DHCPv6/static), IPv6 default gateway, DHCPv6 lease assignments, privacy extension state (RFC 4941), and tunnel adapter inventory (Teredo/6to4/ISATAP); findings for no global address or missing IPv6 gateway.
- **TCP Parameters**: Use `topic: "tcp_params"` for TCP autotuning level, congestion provider (CUBIC/NewReno), initial congestion window, scaling heuristics, dynamic port range, chimney offload state, and ECN capability; findings for disabled autotuning or non-standard congestion provider.
- **WLAN Profiles**: Use `topic: "wlan_profiles"` for saved wireless profiles with authentication type (WPA2/WPA3/WEP/Open), cipher, connection mode, and auto-connect state; currently connected SSID, BSSID, signal, and radio type; findings for profiles using weak/open authentication.
- **IPSec**: Use `topic: "ipsec"` for enabled IPSec connection security rules, active main-mode and quick-mode SAs with local/remote address pairs, IKE Policy Agent service state; findings for active tunnels.
- **NetBIOS**: Use `topic: "netbios"` for NetBIOS over TCP/IP state per adapter (enabled/disabled/DHCP), WINS server configuration, nbtstat registered names and active NetBIOS sessions; findings for enabled NetBIOS and configured WINS servers.
- **NIC Teaming**: Use `topic: "nic_teaming"` for LBFO team inventory (mode, load-balancing algorithm, status), team member operational state; findings for degraded teams or inactive members.
- **SNMP**: Use `topic: "snmp"` for Windows SNMP agent service state, community string presence audit (values redacted), permitted manager list, SNMP Trap service state; findings flag running agents and the 'public' community string as a risk.
- **Port Test**: Use `topic: "port_test"` with `host` and `port` args to test TCP port reachability — returns OPEN/CLOSED/FILTERED with ICMP result, source address, and interface. Example: `inspect_host(topic: "port_test", host: "192.168.1.1", port: 443)`.
- **Network Profile**: Use `topic: "network_profile"` for Windows network location profile per interface (Public/Private/DomainAuthenticated), IPv4/IPv6 connectivity state; findings flag Public-category interfaces.
- **DNS Lookup**: Use `topic: "dns_lookup"` with a required `name` arg for active DNS resolution of a specific hostname — returns A, AAAA, MX, TXT, SRV, or any record type; use `type` arg to specify (default: A). Example: `inspect_host(topic: "dns_lookup", name: "example.com", type: "A")`.
- **IP Config**: Use `topic: "ip_config"` for full adapter IP detail equivalent to `ipconfig /all` — DHCP enabled state, IP addresses, gateway, DNS servers per adapter; useful when you need a complete adapter inventory without DHCP lease timing.
- **Summary**: Use `topic: "summary"` (the default when no topic is given) for a general host overview — OS, hostname, uptime, CPU/RAM snapshot, disk health flag, and active network adapters.
- **Toolchains**: Use `topic: "toolchains"` for installed developer tools — detects Rust, Node, Python, Go, Java, Docker, Git, and other common toolchain binaries with versions.
- **Prompt Synchronicity Rule**: Any addition of an `inspect_host` topic or a new tool **MUST** be reflected in `src/agent/prompt.rs` and the `CAPABILITIES.md` competency matrix. This ensures the agent uses high-precision tools instead of falling back to raw shell.
- **Topic Registration**: When adding a topic to `host_inspect.rs`, synchronize its keywords in `routing.rs` and its mandatory instruction in `prompt.rs` in the same PR.
- **Cross-Platform Parameter Integrity**: When modifying shared tool signatures (especially in `host_inspect.rs`), ensure all parameters are either used on all platforms or explicitly silenced in non-target `#[cfg]` blocks using the `let _ = param;` pattern. This prevents "blindspot" build failures in CI (e.g., breaking Windows by renaming a parameter to satisfy a Unix warning).
- **PATH**: Use `topic: "path"` for PATH entry analysis — lists all entries, flags duplicates, missing directories, and shadowed binaries.
- **Environment Doctor**: Use `topic: "env_doctor"` for a full developer environment health check — PATH sanity, package manager conflicts, toolchain version mismatches, and missing expected tools.
- **Fix Plan**: Use `topic: "fix_plan"` for a grounded, step-by-step remediation plan for a reported issue. Pass `issue` arg with the problem description; the harness inspects relevant machine state and returns an actionable numbered plan.
- **Network Overview**: Use `topic: "network"` for a general network snapshot — adapter list, IP addresses, default gateway, and active connection count.
- **Processes**: Use `topic: "processes"` for running processes ranked by CPU/RAM with PID, name, memory MB, CPU %, and real-time I/O R/W operation counts.
- **Services**: Use `topic: "services"` for Windows/Linux service states — name, status (Running/Stopped), startup type, and description.
- **Ports**: Use `topic: "ports"` for listening TCP/UDP ports — local address, port number, owning process name and PID.
- **Log Check**: Use `topic: "log_check"` for recent system error/warning events from the Windows Event Log or journald — application and system log tails with severity.
- **Startup Items**: Use `topic: "startup_items"` (aliases: `startup`, `boot`, `autorun`) for programs and scripts that run at login — registry run keys, startup folder entries, and scheduled task autorun items.
- **OS Config**: Use `topic: "os_config"` for OS-level configuration — Windows edition, build, activation status, power plan, UAC level, and system locale.
- **Resource Load**: Use `topic: "resource_load"` (aliases: `performance`, `system_load`) for live CPU and RAM utilization with top resource consumers by process.
- **Repo Doctor**: Use `topic: "repo_doctor"` to inspect workspace health — git status, uncommitted changes, branch state, remote tracking, and basic build-file presence (Cargo.toml, package.json, etc.).
- **Disk Benchmark**: Use `topic: "disk_benchmark"` (aliases: `stress_test`, `io_intensity`) for sequential read/write throughput and latency measurements on the workspace drive. Accepts optional `path` arg; falls back to the running binary's drive if the path is not found.
- **Desktop / Downloads**: Use `topic: "desktop"` or `topic: "downloads"` to list files in the user's Desktop or Downloads directory — names, sizes, and modification dates.
- **Disk**: Use `topic: "disk"` with a `path` arg to inspect a specific disk path — free space, filesystem type, and usage.
- **Directory**: Use `topic: "directory"` with a required `path` arg to list any arbitrary directory — file names, sizes, and modification dates.
- **Teacher Mode (`/teach`)**: Activates a grounded walkthrough mode for write/admin tasks Hematite cannot safely execute itself. Protocol: (1) call `inspect_host` with the relevant topic(s) to observe actual machine state, (2) deliver a numbered step-by-step tutorial referencing real observed state — exact commands, exact paths, exact values. Does NOT execute write operations. Covers:
- **SysAdmin Diagnostics**: `inspect_host` (topic=network, services, processes, ports, log_check, startup_items, storage, hardware, updates, security, pending_reboot, disk_health, battery, recent_crashes, scheduled_tasks, connections, etc.)
- **Hardware Diagnostic Suite**: `topic=device_health` (PnP errors), `topic=drivers` (active audit), `topic=peripherals` (USB/HID tree).
- **Deep System Visibility**: `topic=sessions` (Logon sessions), `topic=hardware` (BIOS/Virtualization DNA), `topic=processes` (Real-time I/O tracking), `topic=thermal` (Thermal/Throttling), `topic=activation` (Licensing), `topic=patch_history` (KB Audit), `topic=overclocker` (Precision Silicon Historian).
 matching grounded walkthrough. New lanes: `driver_install`, `group_policy`, `firewall_rule`, `ssh_key`, `wsl_setup`, `service_config`, `windows_activation`, `registry_edit`, `scheduled_task_create`, `disk_cleanup`. Each lane inspects real machine state first, then delivers machine-specific numbered steps.

## Research & Technical Verification

Hematite includes a privacy-first, unlimited research engine powered by a local SearXNG instance.

- **`research_web`**: Search the internet via a local self-healing SearXNG backend. Privacy-first (no identity tracking), unlimited volume (no cloud API rate limits). Use this for technical news, library versions, API updates, and verifying technical claims.
- **`fetch_docs`**: Fetch and convert a URL into a local markdown-ready document for analysis.
- **Proactive Verification**: Hematite is instructed to identify its own knowledge gaps. If a technical fact is not absolute (e.g., "what is the latest version of X"), the agent should use `research_web` to verify before answering.
- **Search Intent Disambiguation**: Queries mentioning "function," "logic," or "repository" are routed to the internal Vein/codebase index. Queries mentioning "latest," "version," "news," or "research" are routed to the web research tool.

## Product Boundary

Hematite is not trying to outscale cloud agents. It is optimized for local GPU task execution.

- Primary target: one RTX 4070-class GPU with roughly 12 GB VRAM
- Main engineering constraints: limited local context, open-model inconsistency, and VRAM pressure under long sessions
- Design response: stronger tooling, grounded traces, compaction, retrieval, and operator workflow instead of pretending the model is smarter than it is

## Behavioral Guidelines

These core guidelines help minimize common LLM coding mistakes. They prioritize caution and precision over speed.

1. **Think Before Coding**: Explicitly state assumptions. Surface tradeoffs and ask for clarification if anything is unclear.
2. **Simplicity First**: Write the minimum code necessary. Avoid speculative abstractions or "future-proofing" that wasn't requested.
3. **Surgical Changes**: Touch only what is necessary. Match existing style and refactor only what is broken. Clean up any artifacts (unused imports/variables) created by your change.
4. **Goal-Driven Execution**: Define clear success criteria (e.g., reproduction tests) and verify every step.

## Product Direction

Hematite should behave like a high-agency coding partner with bounded autonomous lanes.

That means:

- the model handles intent, code judgment, wording, and local reasoning between steps
- the harness handles deterministic workflow structure, recovery, context control, and verification
- autonomy is earned per workflow, not assumed globally

In practice, the product should keep leaning into micro-workflows for recurring task classes:

- startup and UI wording changes
- read-before-edit refactors
- proof-before-edit debugging
- verify-after-mutation coding tasks

When a local model gets uncertain, the answer is usually not "give it more freedom." The answer is tighter scaffolding: narrower tools, better owner-file locking, exact-window inspection, explicit recovery ladders, and honest operator-visible failure states.

**Large-file edit discipline:** Before editing files over ~500 lines (`inference.rs`, `conversation.rs`, `tui.rs`, and similar large modules), recommend `/architect` or a read-only inspection pass first unless the user has already provided a clear plan or target line range. Direct `/code` on large files without orientation leads to missed context and off-target edits on 9B models. This applies to any large codebase the user runs Hematite against, not just Hematite's own source.

## MCP Server Mode

Hematite can run as an MCP server, exposing its 116+ host inspection tools to any MCP-capable agent over the stdio transport.

```powershell
hematite --mcp-server
```

This starts a JSON-RPC 2.0 newline-delimited stdio server. No TUI launches. Protocol stays on stdout; all logging goes to stderr.

**Claude Desktop configuration** (`~/.claude/claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "hematite": {
      "command": "hematite",
      "args": ["--mcp-server"]
    }
  }
}
```

**Tool exposed:** `inspect_host` — all 116+ topics, same as the TUI. Any MCP-capable client (Claude Desktop, OpenClaw, Cursor, Windsurf) can call it directly and get grounded machine state with no cloud, no API key, and no prompt guessing.

**Implementation:** `src/agent/mcp_server.rs` — stdio reader loop, JSON-RPC dispatch, delegates to `crate::tools::host_inspect::inspect_host()`.

### Edge Redaction (Tier 1 — Regex)

Add `--edge-redact` to strip sensitive identifiers before any response leaves the machine:

```powershell
hematite --mcp-server --edge-redact
```

Patterns sanitized before the cloud agent sees the output:
- Usernames in file paths (`C:\Users\<name>\` → `C:\Users\[USER]\`)
- MAC addresses → `[MAC]`
- Hardware / disk serial numbers → `[SERIAL]`
- Hostnames and computer names → `[HOSTNAME]`
- AWS access key IDs → `[AWS-KEY]`
- Credential-shaped env values (API keys, tokens, passwords) → `[REDACTED]`

Every response includes a receipt header the cloud model can read:
```
[edge-redact: 4 substitution(s) — username-path ×4 — values replaced before leaving this machine]
```

**Use case:** enterprises and security-conscious operators who want frontier model reasoning (Claude Desktop, OpenClaw) without raw machine identifiers crossing the wire. The local machine provides grounded observations; the cloud model reasons about them — but never sees the raw identity data.

**Implementation:** `src/agent/edge_redact.rs` — `lazy_static!` compiled regex patterns, `redact()` returns count by category, `apply()` prepends the receipt header.

### Semantic Redaction (Tier 2 — Local Model Summarizer)

Add `--semantic-redact` and `--semantic-model` to route inspect_host output through a dedicated local model before any data leaves the machine:

```powershell
hematite --mcp-server --semantic-redact --semantic-model bonsai-8b
```

The summarizer model receives raw diagnostic output and produces an anonymous diagnostic summary — stripping usernames, hostnames, MACs, local IPs, serial numbers, org names, and credentials while preserving diagnostic value (versions, error codes, metrics, findings, time deltas). Tier 1 regex runs after the semantic pass as a final safety net to catch anything the model missed.

**`--semantic-model`** specifies which model in LM Studio handles privacy summarization. This is separate from the main reasoning model — it only activates during MCP calls with `--semantic-redact`. The main TUI model (Qwen etc.) is never involved in privacy filtering. When multiple models are loaded in LM Studio, this flag is required.

**`--semantic-url`** (optional) points the summarizer at a different server endpoint entirely — useful if running the privacy model on a separate llama.cpp instance or a second LM Studio installation. If omitted, the summarizer uses the same port as `--url` (default: `http://localhost:1234/v1`). All three models (main + embed + summarizer) can share port 1234 in LM Studio's multi-model mode.

**Choosing a summarizer model:** any instruction-following model works. Smaller is better for constrained setups since the summarizer runs alongside your main model:
- **RTX 4070 (12 GB):** [Bonsai 8B Q1_0](https://huggingface.co/prism-ml/Bonsai-8B-gguf) at 1.16 GB — verified. Fits with Qwen3.5 9B + nomic-embed, 8.22 GB total.
- **RTX 4080/4090 (16–24 GB):** any 8B Q4_K_M model at 5–6 GB — better summarization quality.
- **Workstation / multi-GPU:** 70B-class models — near-perfect identity stripping.

The summarizer does not need to be good at code or reasoning. Benchmark scores for instruction-following and summarization matter; coding benchmarks do not.

**Fail-safe:** if the local model is unreachable, the tool call returns an error — raw data is never sent to the cloud model.

**Jailbreak resistance:** the summarizer prompt is injected by Hematite and wraps system data in `<diagnostic_data>` tags explicitly marked as untrusted. Unknown MCP arguments are stripped before tool dispatch. Model refusals are detected and treated as errors.

**Audit trail:** every tool call is logged to `~/.hematite/redact_audit.jsonl` with metadata only (topic, mode, substitution counts, input/output size, shrink ratio). Raw output and original values never appear in the audit log.

**Implementation:** `src/agent/semantic_redact.rs` — HTTP to LM Studio `/v1/chat/completions`, temperature=0, max_tokens capped at 1.5× input length. `src/agent/redact_audit.rs` — JSONL appender, no external deps.

### Redaction Policy File

Create `.hematite/redact_policy.json` (workspace) or `~/.hematite/redact_policy.json` (global) to control per-topic behavior:

```json
{
  "blocked_topics": ["user_accounts", "credentials", "audit_policy"],
  "allowed_topics": [],
  "topic_redaction_level": {
    "network": "semantic",
    "hardware": "regex"
  },
  "default_redaction_level": "regex"
}
```

- `blocked_topics` — MCP returns an error for these topics; the inspection never runs
- `allowed_topics` — if non-empty, only these topics are served (whitelist mode)
- `topic_redaction_level` — override redaction level per topic: `"none"`, `"regex"`, or `"semantic"`
- `default_redaction_level` — fallback when no per-topic override exists

See `.hematite/redact_policy.example.json` for a full template. Workspace config overrides global.

**Implementation:** `src/agent/redact_policy.rs` — loaded once at MCP server startup.

## MCP Configuration

Hematite loads stdio MCP servers from:

- `~/.hematite/mcp_servers.json`
- `.hematite/mcp_servers.json`

Workspace config overrides global config by server name. On Windows, wrapper launchers such as `npx`, `npm`, `.cmd`, and `.bat` are resolved automatically.

## LLM Provider Configuration

Hematite defaults to LM Studio on `http://localhost:1234/v1`. To use a different OpenAI-compatible server (Ollama, vllm, a remote machine, etc.), set `api_url` in `.hematite/settings.json`:

```json
"api_url": "http://localhost:11434/v1"
```

This overrides the `--url` CLI flag. The value must be the base `/v1` path — Hematite appends `/chat/completions`, `/models`, and `/embeddings` automatically.

Common values:
- LM Studio (default): `http://localhost:1234/v1`
- Ollama: `http://localhost:11434/v1`
- Remote machine: `http://192.168.x.x:1234/v1`

**Global settings fallback.** Hematite merges two config files at startup: the workspace-level
`.hematite/settings.json` (inside the project root) and the global `~/.hematite/settings.json`
(in the user's home directory). Workspace values always win; global fills in any fields not set
by the workspace. This means `api_url`, `model`, `voice`, and other preferences set globally apply
in every directory — including non-project launches from the desktop or home folder. The workspace
config is created automatically on first run in a new directory.

**Workspace profile.** Hematite writes `workspace_profile.json` into the active runtime-state
directory on startup. In normal project workspaces that is `.hematite/workspace_profile.json`; in OS shortcut directories such as Desktop or Downloads it falls back to `~/.hematite/workspace_profile.json`
so no local `.hematite/` folder is created there. The file is auto-generated and gitignored when
local. It contains detected stack/package-manager hints, important folders, ignored noise folders,
and build/test suggestions. The prompt can use it as lightweight grounding before the model starts
guessing about repo shape. Use `/workspace-profile` to inspect the current generated profile in the TUI.

## Model Compatibility Notes

**Jinja template fix — `| safe` filter error:** Some bartowski quantizations (e.g. `qwen_qwen3.6-35b-a3b` IQ2_XXS) ship with a broken Jinja chat template that LM Studio cannot render. Symptom: `Unknown StringValue filter: safe` channel errors after the first tool call.

Fix: In LM Studio, open the model → **Prompt Template → Template (Jinja)** tab. Find this line:

```jinja
{%- set args_value = args_value | string if args_value is string else args_value | tojson | safe %}
```

Change it to:

```jinja
{%- set args_value = args_value | string if args_value is string else args_value | tojson %}
```

Save. The model will work correctly after this one-character fix.

**Primary target model:** `Qwen/Qwen3.5-9B Q4_K_M` on LM Studio. Larger models at extreme quantizations (IQ2_XXS) often have worse effective instruction-following than the 9B at Q4_K_M and are not recommended for Hematite's tool-routing patterns.

## API Configuration

Hematite uses Jina Reader/Search for web research. You can run without a key on the public tier, but a key is recommended for stability.

1. Get a key at [jina.ai](https://jina.ai).
2. Set `JINA_API_KEY`.
3. Or create a local `.env` file with `JINA_API_KEY=...`.

## Architecture

```text
src/
  main.rs               Entry point. Wires channels, spawns tasks, launches the TUI.
  agent/
    inference.rs        InferenceEngine: HTTP to LM Studio, streaming, tool calls.
    conversation.rs     ConversationManager: turn loop, tool dispatch, prompt assembly.
    swarm.rs            SwarmCoordinator: parallel worker agents.
    specular.rs         Watcher and side-panel event source.
    mcp.rs              MCP transport and framing.
    mcp_manager.rs      MCP server lifecycle and discovery.
    prompt.rs           System prompt builder and workspace rule injection.
    parser.rs           Tool call parsing.
    transcript.rs       Session transcript serialization.
    git.rs              Git helpers.
    config.rs           Runtime config loading.
    compaction.rs       Context compaction and summarization helpers.
  tools/
    mod.rs              Tool registry and dispatch.
    file_ops.rs         File listing, reading, writing, project mapping.
    file_edit.rs        Targeted editing helpers.
    shell.rs            Shell execution.
    git.rs              Git tool implementations.
    lsp.rs / lsp_tools.rs  LSP startup and language-aware tooling.
    verify_build.rs     Build validation tool.
    guard.rs            Safety checks for risky actions.
  ui/
    tui.rs              Main TUI loop, rendering, input handling.
    voice.rs            VoiceManager and local TTS pipeline.
    gpu_monitor.rs      Background VRAM polling.
    modal_review.rs     Swarm diff review modal.
    hatch.rs            Rusty personality generation.
  memory/
    vein.rs             Vein RAG: SQLite FTS5 BM25 + semantic embedding retrieval.
    repo_map.rs         PageRank-powered structural overview of the codebase.
    deep_reflect.rs     Idle-triggered session memory synthesis.
libs/
  kokoros/              Vendored voice synthesis library.
```

## Voice Engine

Hematite ships a fully self-contained TTS pipeline using the vendored Kokoro engine. No cloud, no
separate install, no Python — everything is baked into the binary at compile time.

**How it works:**

- The Kokoro ONNX model (`kokoro-v1.0.onnx`, 311 MB) and voice styles (`voices.bin`, 27 MB) are
  embedded in the binary via `include_bytes!` at compile time
- ONNX Runtime 1.24.2 is **statically linked** via `ort`'s `download-binaries` feature — the
  system `onnxruntime.dll` is never used, eliminating DLL version conflicts
- `DirectML.dll` (GPU inference on Windows) ships alongside the binary — copied to `target/debug/`
  by the build, bundled in portable releases
- 54 voices are available across English (American/British), Spanish, French, Hindi, Italian,
  Japanese, and Chinese — all baked in, no downloads at runtime
- Voice ID, speed (0.5–2.0×), and volume (0.0–3.0×) are configurable via `/voice` or `settings.json`

**First-start note:** ONNX graph optimization runs on first load, which takes 10–30 seconds on an
RTX 4070-class system. Subsequent starts reuse the optimized graph. During loading, incoming speech
tokens buffer (1024 capacity) so no audio is lost.

**Why static linking matters:** Windows ships `onnxruntime.dll` 1.17 in System32. Kokoro's ONNX
model uses opsets not supported by 1.17. Dynamic loading would silently crash inside C code before
any Rust error handler could catch it. Static linking with 1.24.2 sidesteps this entirely — the
binary carries the exact runtime it was built against.

**Runtime DLL footprint:** only `DirectML.dll` is needed alongside the binary. It ships with
Windows 10 1903+ and is also bundled in the Hematite portable release.

## Key Concepts

- `InferenceEvent`: the enum flowing from agent to TUI over `mpsc`
- Thought routing: model reasoning is routed to the side panel instead of the main chat
- `SPECULAR` panel: shows live reasoning, recent reasoning trace, and watcher events
- `ACTIVE CONTEXT`: shows the current working file set
- Ghost system: `.hematite/ghost/` stores pre-edit backups
- Hardware guard: `gpu_monitor.rs` watches VRAM and can force brief mode or reduce swarm fanout
- Startup greeting prints active endpoint (`Endpoint: http://localhost:1234/v1`) so misconfigured providers are immediately visible

## The Vein — Local RAG

The Vein is Hematite's retrieval-augmented generation layer. At the start of each turn it indexes
any changed files and queries for context relevant to the user's message. Results are injected into
the system prompt so the model starts with the right code already in view, reducing tool calls.

**Per-workspace database:** stored in the active runtime-state directory as `vein.db`. In normal
project workspaces that means `.hematite/vein.db`; in OS shortcut directories it falls back to
`~/.hematite/vein.db`. Each real project folder still gets its own index. The Vein learns from
files on disk and local session artifacts, not from cloud state.

**Non-project directories:** when Hematite is launched outside a real project (no `Cargo.toml`,
`package.json`, `go.mod`, etc. found walking up from the launch directory), it skips the source-file
walk but still keeps The Vein active in docs-only mode. `docs/`, imported chats in `imports/`, and
recent local session reports in the active runtime-state directory remain searchable, and the status
badge shows `VN:DOC`. A bare `.git` alone does not count
as a project workspace.

**Auxiliary local memory inputs:** besides project source, The Vein also indexes:

- `.hematite/docs/` for permanent local reference material
- the runtime-state `reports/` directory for recent local session reports, chunked by exchange pair (`user` +
  `assistant`) and capped to the last 5 sessions / 50 turns per session
- `.hematite/imports/` for imported chat exports (Claude Code JSONL, Codex CLI JSONL, simple
  role/content JSON, ChatGPT-style `mapping` exports, or `>` transcripts), also chunked as
  session memory without inflating source/doc status counts

**Two retrieval modes, hybrid-merged:**

- **BM25** (always available) — SQLite FTS5 full-text search with Porter stemming. Fast, zero GPU
  cost, works even when LM Studio has no embedding model loaded.
- **Semantic** (optional, higher quality) — Calls `/v1/embeddings` on LM Studio to embed each chunk
  using `nomic-embed-text-v2` Q8_0. Understands synonyms and concept-level matches; finds "what
  renders on startup" even when no file uses the word "banner". Vectors are stored in SQLite so they
  survive restarts without re-embedding.

**To enable semantic search:** load `text-embedding-nomic-embed-text-v2` in LM Studio alongside
your main coding model. On an RTX 4070 this costs ~512 MB VRAM — both models fit comfortably.
Status bar shows `VN:SEM` (green) when active, `VN:FTS` (yellow) for BM25-only project/docs
indexing, and `VN:DOC` when only docs/session memory are active outside a project.

**Automatic backfill:** if the embedding model is loaded after initial indexing, Hematite detects
unembedded chunks and fills them gradually (20 per turn) without needing a reset or file-touch.

**How hybrid ranking works:** semantic hits score 1.0–2.0 (preferred), BM25 fills to 0.0–1.0 for
paths not already covered. Results are deduplicated by file path and capped at 1500 chars total.

**Active-room bias:** file edit heat is tracked per path. The hottest subsystem room gets a small
retrieval boost, and a compact hot-files block grouped by room is injected into the prompt so the
model stays oriented toward the part of the codebase you're actively editing.

**L1 hot-files context:** the top 8 hottest files (by edit count) are grouped by room and injected
as a compact block near the top of the system prompt every turn. This gives the model immediate
structural orientation — which subsystems are active — before it reads any retrieval results or
repo map output. Returns `None` and injects nothing on a fresh project with no heat records.

**PageRank Repo Maps:** at startup and after every file edit, Hematite builds a `tree-sitter` definition/reference graph across all source files and runs PageRank (via `petgraph`) to rank files by structural importance. The ranked map is injected into the system prompt so the model immediately knows which files are architecturally central — no tool calls needed for basic orientation. Hot-file personalization uses heat-weighted scores from The Vein: the hottest file gets a 100× boost; others scale proportionally (e.g. half the edits → 50× boost). This means files that are both architecturally central *and* actively edited float to the top.

**Ranking cues:** reranking adds small boosts for exact quoted phrases, standout tokens such as
filenames/commands/tool IDs, "what did we decide earlier" style prompts that should prefer
session/import memory over generic source overlap, and time-anchored memory prompts such as
explicit dates, "yesterday", or "last week" so the right session period outranks stale matches.

**Room taxonomy:** room detection is also rule-based across path segments and filenames now, so
runtime/config/release/integration/doc files do not all collapse into generic folder labels.

**Memory-type tagging:** session-room chunks (local session reports and imported chat exports) are classified by `detect_memory_type(text)` — a zero-cost regex pass in `src/memory/vein.rs` that tags each chunk as `decision`, `problem`, `milestone`, `preference`, or `""`. The tag is stored in `chunks_meta.memory_type`. `QuerySignals::from_query` sets `query_memory_type` by detecting intent words in the query. During retrieval, matching chunks receive a `+0.35` score boost via `retrieval_signal_boost`. This lets session memory surface by intent (an architectural decision vs. a reported bug vs. a style preference) without the operator using special syntax.

**Operator inspection:** `/vein-inspect` prints a compact report of the current Vein state:
workspace mode, indexed source/docs/session counts, embedding availability, active room bias, and
the current hot files grouped by room. Use it when you want to inspect what memory Hematite is
actually carrying.

**Incremental indexing:** files are re-indexed only when their mtime changes. BM25 runs on every
changed file; embeddings are generated for the same files so the vector store stays in sync.

**Chunking strategy:** Rust files are split at symbol boundaries (fn/impl/struct/enum boundaries),
keeping doc-comments with their item. Other files split at paragraph breaks. Oversized blocks
fall back to a sliding window. This ensures each retrieved chunk is a coherent, complete code unit.

**Resetting the index:** `/vein-reset` wipes all three tables and resets the status badge to
`VN:--`. The next turn rebuilds from scratch. `pwsh ./clean.ps1 -Deep` also deletes the DB file.

**File size limit:** 512 KB per file. Large files like `tui.rs`, `inference.rs`, and `conversation.rs` are indexed in full. Files over 512 KB are skipped.

**BM25 query shape:** stopwords are stripped and tokens are OR-joined in the FTS5 query. This prevents conversational queries like "how does the specular panel work" from returning zero results due to FTS5 implicit AND semantics.

**Backfill ordering:** `.rs` files are embedded first so the most relevant source files get semantic vectors before documentation or config files.

## Model Behavior Notes

- Some local models omit an opening reasoning tag; the streamer handles this
- Some local servers return `tool_calls: []` instead of `null`; Hematite filters this
- Conversation history slices must start with a `user` message for LM Studio/Jinja alignment
- Tool hallucination guards block fake tool names such as `thought` or `reasoning`
- Gemma 4: tool results are wrapped in `<|tool_response>response:{name}{...}<tool_response|>` native markup; controlled by `gemma_native_auto` / `gemma_native_formatting` config
- Gemma 4: messages are wrapped with `<|turn>` markup before sending; non-Gemma models must NOT receive this wrapping
- Standard models (Qwen, etc.): tool results use plain content; no model-specific markup applied
- Standard models (Qwen, etc.): jinja templates require exactly one `system` role message — a second system message causes a 400 Channel Error; `loop_intervention` is merged into `history[0]` instead of appended
- Turn-level transient retry budget (3 per turn) caps runaway retry loops on Channel Errors; budget resets on successful inference
- Repeat guard: if the same `(tool_name, args)` is called 3+ times in a turn, a hard stop intervention is injected; `verify_build` and git tools are exempt (fix-verify loops are legitimate)
- Naked reasoning prose leaked without `<think>` tags is stripped from visible output before it reaches chat; stray `</think>`, `</function>`, `</tool_call>`, and similar XML artifacts are also stripped
- `edit_file` and `multi_search_replace` normalize CRLF → LF before matching so model search strings (always LF) work correctly on Windows files; on exact-match failure, Hematite escalates through: (1) rstrip-only match — strips trailing whitespace, preserves indentation; (2) full-strip match — strips all surrounding whitespace; if both fail, scans up to 100 workspace source files for the search string and names the matching file in the error message (cross-file hint); on any fuzzy match, replace-string indentation is delta-corrected automatically
- Diff preview: before `edit_file`, `patch_hunk`, or `multi_search_replace` is applied, a coloured before/after diff modal is shown in the TUI; user presses Y to apply or N to skip; model is told "Edit declined by user." on N; bypassed in `--yolo` mode
- Tool output overflow: when a tool result exceeds 8 KB, `cap_output_for_tool` writes the full text to `.hematite/scratch/<tool>_<timestamp>.txt` inside the active runtime-state directory and returns a truncation notice with the scratch path; the model recovers the full content with `read_file` without repeating the original tool call; large `read_file` results under compact-context mode follow the same scratch path
- `read_file` satisfies the line-inspection grounding check so the model can go `read_file → edit_file` without a separate `inspect_lines` call
- Context compaction warnings fire as visible System messages at 70% and 90% context fill; the warning resets below 60% so it only fires once per pressure band
- Embed model load/unload is detected mid-session: when LM Studio swaps the embedding model (or unloads it), Hematite fires a System message in the TUI immediately so the operator knows semantic search state changed
- Startup CWD guard: if Hematite is launched from an inaccessible system folder (e.g. via a Windows shortcut pointing to a system path), it silently relocates to the user's home directory before any workspace detection runs, preventing a hung startup

## Commit Style

Use lowercase conventional commits:

```text
feat: add X
fix: correct Y
refactor: restructure Z
chore: update deps / clean repo
docs: update README
```

## Session Economics and Reporting

Hematite tracks token usage and session cost in real time.

- Exit (Ctrl+C) and cancel (ESC) flows copy the session transcript to the clipboard
- Session reports are written to `reports/session_YYYY-MM-DD_HH-MM-SS.json` under the active runtime-state directory on every exit and cancel
- Report includes: session start timestamp, duration, model, context length, total tokens, estimated cost, turn count, and full transcript
- The runtime-state `reports/` directory is gitignored when local — reports are local runtime artifacts
- The Vein indexes recent reports as local retrieval memory by exchange pair, capped to the last 5 sessions and 50 turns per session, tagged as `session` room memory so they do not pollute normal source-file status counts
- `.hematite/imports/` is the manual cross-tool memory lane: drop useful exported chats there and
  Hematite will index them automatically as imported session exchanges on the next pass

## Sandboxed Code Execution

Hematite exposes a `run_code` tool that lets the model write and run JavaScript/TypeScript or Python in a restricted subprocess. This is real execution — the model gets actual output, not training-data approximations.

**Deno sandbox (JS/TS):**
- Flags: `--deny-net --deny-env --deny-sys --deny-run --deny-ffi --allow-read=. --allow-write=. --no-prompt`
- Code fed via stdin — no temp file created or cleaned up
- `NO_COLOR=true` set so output is clean

**Python sandbox:**
- `env_clear()` + blocked socket, os.system, os.popen, and dangerous module imports (subprocess, urllib, requests, etc.) via a custom `__import__` wrapper
- Note: Python sandboxing is best-effort (no OS-level permission flags like Deno)

**Both runtimes:**
- Hard timeout: 10 seconds default, up to 60 seconds if the model passes `timeout_seconds`
- 16 KB output cap (8 KB stdout + 8 KB stderr)
- Clear error message if the runtime is not installed — no silent failure

**Runtime detection order for Deno:** `~/.lmstudio/.internal/utils/deno.exe` (LM Studio's bundled copy, present for all LM Studio users) → system `deno` on PATH. Since Hematite requires LM Studio, JS/TS execution works with zero install for every user.

**Runtime detection for Python:** `python3` → `python` on PATH. Python 3 ships with Windows 11 and most machines.

**To install Deno system-wide** (optional, for use outside Hematite): `winget install DenoLand.Deno`.

**Computation Integrity Routing:** Hematite automatically detects when a query requires precise numeric computation and nudges the model to reach for `run_code` instead of answering from training-data memory. Detection categories: checksums/hashes (SHA, MD5, CRC), financial/percentage calculations, statistical analysis (mean, std dev, regression), unit conversions (bytes, temperature, distance, weight), date/time arithmetic (days between dates, Unix timestamps), algorithmic verification (prime checks, sorting, factorial), and any explicit "run this code" request. When detected, a pre-turn COMPUTATION INTEGRITY NOTICE is injected so the model computes the real result rather than guessing. Two harness-level recovery paths back this up: if the model attempts `shell` for sandbox-style execution, it is blocked and forced to retry with `run_code`; if the model writes Python without specifying `language: "python"` and Deno rejects the syntax, the harness detects the parse error and forces a corrective retry with the correct language. The routing logic lives in `src/agent/routing.rs` (`needs_computation_sandbox`); the recovery interventions live in the tool result handler in `src/agent/conversation.rs`.

## Document and Image Attachments

Hematite supports attaching files to any conversation turn via hotkeys or slash commands.

**Document attachment (`Ctrl+O` / `/attach <path>`):**
- Supported types: PDF (text-based), markdown, plain text
- PDF extraction is best-effort using pure-Rust `pdf-extract` — works for standard PDFs (Word exports, LaTeX, API docs); rejects with a clear error if words are smashed together or text is too short (common with academic publisher PDFs using custom embedded fonts like EBSCO, Elsevier, Springer)
- Size feedback: after loading, Hematite estimates the token cost (chars/4) and warns if the attachment exceeds 40% of the active context window (yellow warning) or 75% (red warning), so the operator knows before sending
- Permanent indexing: drop files in `.hematite/docs/` and the Vein indexes them alongside source code — hybrid BM25+semantic retrieval, no separate step required
- One-shot: `/attach` injects content as a context prefix on the next message then clears

**Image attachment (`Ctrl+I` / `/image <path>`):**
- Supported types: PNG, JPG, JPEG, GIF, WebP
- Encoded as a base64 data URL and passed to the model via the multimodal vision path
- Works with any vision-capable model loaded in LM Studio
- Useful for: screenshots of bugs, UI mockups, architecture diagrams, scanned documents that PDF extraction can't handle

**Clearing attachments:**
- `/detach` drops any pending document or image before sending
- Attachments are cleared automatically after the next turn

## Versioning Policy

Hematite follows [Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`).

| Bump | When |
|---|---|
| `PATCH` (0.1.**1**) | Bug fixes, doc updates, internal refactors with no user-visible change |
| `MINOR` (0.**2**.0) | New user-visible features, meaningful UX improvements, new tools |
| `MAJOR` (**1**.0.0) | Breaking config/API changes, or the first stable public release |

**Pre-1.0 rule:** while the version is `0.x.y`, minor bumps are used freely for new features. Don't stay on a patch version just because the change feels small — if a user would notice it, it's a minor bump.

**When to bump:**
- Never bump mid-development. Version numbers live in `Cargo.toml` and are baked into the binary at compile time.
- `Cargo.toml` is the Rust package manifest and the release version source of truth. Other release surfaces are updated to match it.
- For unreleased work, validate the change in a rebuilt local portable first: `pwsh ./scripts/package-windows.ps1 -AddToPath`, restart the terminal, and test the live behavior.
- Bump only after the feature work is committed and the local portable has already proven the behavior. Do not bump just to test whether a fix might work.
- Always use `bump-version.ps1` — never edit version strings by hand across files.
- `bump-version.ps1` now self-verifies the static release surfaces immediately after replacement. After `cargo build`, run `pwsh ./scripts/verify-version-sync.ps1 -Version X.Y.Z -RequireCargoLock` before committing the bump.
- After bumping, run `cargo build` (this also regenerates `Cargo.lock`), then commit **exactly these five files** and nothing else:
  ```
  git add Cargo.toml Cargo.lock README.md CLAUDE.md installer/hematite.iss
  git commit -m "chore: bump version to X.Y.Z"
  ```
  Never use `git add .` for a bump commit — it can sweep in unrelated changes. Never skip `Cargo.lock` — it must match `Cargo.toml`.

**Commit message for a version bump:**
```
chore: bump version to X.Y.Z
```

## Release Build

**Recommended wrapper for routine releases:**

```powershell
pwsh ./release.ps1 -Version X.Y.Z
```

For solo use, prefer `release.ps1` over manually retyping the release sequence. It refuses to run from a dirty worktree, sets the exact release version when you use `-Version X.Y.Z`, rebuilds `Cargo.lock`, verifies version sync, commits the version files, creates the annotated tag, then builds release artifacts from that tagged commit. Add `-Push` to also push `main` and the tag automatically. Use `-Bump patch|minor|major` when you want the script to calculate the next semantic version for you.

`pwsh ./release.ps1 -Version X.Y.Z -AddToPath -Push` is the full Windows publish path: local bump commit, local tag, rebuilt portable bundle, rebuilt installer, PATH update, then push of both `main` and the new tag.

That order is intentional. Hematite's startup banner and `/version` only show `release` when the binary is compiled from the exact matching tag, so local release artifacts must be built after the tag exists.

For crates.io automation:

- add `-PublishCrates` to publish `hematite-cli` after the push succeeds
- add `-PublishVoiceCrate` only when `hematite-kokoros` changed and must be published first
- `-PublishCrates` requires `-Push`; do not publish crates from a local-only release state

**Practical operator order:**

1. Land the actual feature or fix first.
2. Add or update diagnostics coverage when the change introduces or materially changes behavior.
3. Rebuild the local Windows portable without bumping:
   `pwsh ./scripts/package-windows.ps1 -AddToPath`
4. Restart the terminal, run the local portable, and test the live behavior.
5. Commit the feature work as a normal commit.
6. When the work is proven, run `pwsh ./release.ps1 -Version X.Y.Z -AddToPath -Push` or the appropriate `-Bump` variant from a clean tree.
7. **Wait for CI to go green on both Windows and Linux** before publishing to crates.io. Pushing the tag triggers both release workflows. If either fails, push a patch fix first — never publish a crate from a state where CI is red on any platform.

Do not bump just to test whether a feature works. For Hematite, the local portable is the pre-release smoke test. Public version bumps happen after the live local test passes.

`release.ps1` is for cutting a release from a known-good state. It is not a substitute for first validating an unshipped fix in the local portable.

For behavioral changes, diagnostics are part of the change, not optional cleanup. Prefer adding or updating focused coverage in `tests/diagnostics.rs` as you land the work so the live portable test is not your only proof.

**Solo verification loop (Codex/operator path):**

```powershell
cargo fmt
cargo check --tests
cargo test --test diagnostics
powershell -ExecutionPolicy Bypass -File ./scripts/verify-doc-sync.ps1
pwsh ./scripts/package-windows.ps1 -AddToPath
```

Why these exist:

- `cargo fmt`
  Normalizes Rust formatting so the diff stays readable and consistent.
- `cargo check --tests`
  Fast compile check for both app code and test code without paying the full release-build cost yet.
- `cargo test --test diagnostics`
  Runs the focused behavior checks where tool routing, Vein behavior, host inspection, and other product-level regressions are usually covered.
- `pwsh ./scripts/package-windows.ps1 -AddToPath`
  Rebuilds the actual portable build you run locally, updates the PATH-backed copy, and gives you the real pre-release smoke test.

When the change is narrow, prefer a targeted diagnostics test instead of the full file:

```powershell
cargo test --test diagnostics test_name_here -- --exact
```

**Routing fix workflow (inspect_host topic routing gaps):**

When a query routes to shell instead of `inspect_host`, the fix pattern is:

1. Check `preferred_host_inspection_topic()` in `src/agent/routing.rs` — if the topic has no `asks_*` variable there, that's the root cause. `host_inspection_mode` is derived from this function; if it returns `None`, the HOST INSPECTION MODE system prompt is never injected and the model free-forms.
2. Add the missing `asks_*` variable with natural-language phrases that cover the query shape.
3. Add it to the dispatch chain (`if asks_X { Some("topic") }`).
4. Update the HOST INSPECTION MODE bullet list in `src/agent/conversation.rs` to include the new topic so the model knows to use it.
5. Add a `test_routing_detects_*_topic` test in `src/agent/conversation.rs` (the current tests live there) covering 2–3 representative phrases.
6. Run `cargo test --lib agent::conversation::tests`, rebuild portable, test the live query.

Note: `all_host_inspection_topics()` (used for multi-topic harness pre-runs) is a separate table — a topic can be in one and not the other. Always check `preferred_host_inspection_topic()` specifically.

Inside Hematite itself, explicit cleanup, local packaging, and scripted release requests should prefer the structured approval-gated Hematite maintainer workflow tool instead of falling back to raw shell. Use that path when the user is asking to run Hematite's own `clean.ps1`, `scripts/package-windows.ps1`, or `release.ps1` in natural language. Do not present it as a generic current-workspace script runner.

For project-specific questions or commands, launch Hematite in the target project directory before asking. Hematite's own maintainer workflows are separate from whatever scripts exist in the current workspace.

Launching from the home directory is valid for workstation inspection, docs-only memory, and general machine questions. It is not the right default for project-specific build, test, script, or repo work.

For normal project work, prefer the separate workspace workflow lane for the active repo's build, test, lint, fix, package scripts, make/just/task targets, local repo scripts, or exact project commands. That path is rooted to the locked workspace, not to Hematite's own source tree.

For a new contributor or non-technical operator, the short explanation is: format the code, make sure it still compiles, make sure the behavior test passes, then rebuild the real app and try it live.

**Step 1 — bump the version** (updates tracked release metadata and verifies the static surfaces):

```powershell
pwsh ./bump-version.ps1 -Version X.Y.Z
```

Never edit version numbers by hand — they will drift across files.

**Step 2 — rebuild the lockfile and verify the full version state:**

```powershell
cargo build
pwsh ./scripts/verify-version-sync.ps1 -Version X.Y.Z -RequireCargoLock
```

**Step 3 — tag and push to trigger CI:**

```powershell
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main
git push origin vX.Y.Z
```

Pushing the tag triggers `windows-release.yml` and `unix-release.yml` on GitHub Actions. Both workflows download the Kokoro voice model assets, run `cargo build --release`, package the artifacts, and attach them to the GitHub Release automatically when they go green. No manual upload needed.

**Local build (optional, for testing before tagging):**

```powershell
pwsh ./scripts/package-windows.ps1
```

- The ONNX model (311 MB) is baked into the binary at compile time — no separate download
- `DirectML.dll` is copied from `target/release/` automatically by the ORT build script
- Output: `dist/windows/Hematite-X.Y.Z-portable.zip` (~336 MB)
- `dist/` is gitignored — these are release artifacts, not tracked in source

## Cleanup

```powershell
pwsh ./clean.ps1           # ghost, scratch, memories, sandbox, reports, logs
pwsh ./clean.ps1 -Deep    # + target/, onnx_lib/, vein.db
pwsh ./clean.ps1 -Deep -PruneDist   # + old dist/ artifacts, keeps only current Cargo.toml version
pwsh ./clean.ps1 -Reset   # + PLAN.md, TASK.md (full blank-slate, simulates new user)
```

Regular clean removes runtime artifacts: ghost backups, scratch files, session memories, sandbox output, reports, and logs (`.hematite/logs/`). Deep also removes build outputs and the vein database. Note: session logs previously written to `.hematite_logs/` in the project root now live at `.hematite/logs/` — delete any leftover `.hematite_logs/` directories manually. `-PruneDist` is opt-in and removes stale packaged artifacts under `dist/` while keeping only the current `Cargo.toml` version. Reset goes further and wipes session state files — use this to simulate a first-run experience without touching `settings.json` or `mcp_servers.json`.

For Hematite, disk growth is a normal maintenance concern. This is a heavy native Rust project with release packaging, ORT/DirectML sidecars, tests, and repeated debug/release builds. `target/` can climb into the tens of gigabytes quickly, and after enough iteration it is believable to hit 50-100 GB of local build output. Treat periodic deep cleanup as part of the normal workflow. When disk pressure matters, run `pwsh ./clean.ps1 -Deep`; if you also want to keep only the latest packaged release artifacts, use `pwsh ./clean.ps1 -Deep -PruneDist`. Remember that the next full rebuild will be slower because you deliberately wiped cached build state.

## Contributor Roadmap

Hematite is designed around the real constraint of a single consumer GPU running 9B-class open models. The goal is not to pretend the local model is smarter than it is. The goal is to make the harness so tight that a 9B model on a 4070 can do real work.

This roadmap reflects that design philosophy: things that are worth doing now because they work with the model's actual capability, and things to revisit when local models improve.

### Shipped

- **Streaming shell output** — ✓ Done. `execute_streaming` streams each stdout/stderr line to the SPECULAR panel as it arrives via `InferenceEvent::ShellLine`. `verify_build` uses the same path. Background tasks fall back to blocking execution.
- **Turn checkpointing** — ✓ Done. `save_session()` writes `.hematite/session.json` after every turn. On next startup, `load_checkpoint()` surfaces the resume hint in SPECULAR and `running_summary` + `session_memory` are reinjected into the model’s system prompt. `/new` and `/forget` both clear the session cleanly via `save_empty_session()`.
- **Computation integrity routing** — ✓ Done. `needs_computation_sandbox()` in `routing.rs` detects math queries and injects a pre-turn nudge. Shell-to-run_code block recovery and Deno parse error recovery are wired in the tool result handler.
- **Per-project rule injection** — ✓ Done. Hematite now natively checks for `.hematite/rules.md` and `HEMATITE.md` (alongside `CLAUDE.md`) and injects them into the system prompt. Includes a new `/rules` command for viewing and editing guidelines directly from the TUI.
- **Ultra-Deterministic Teleportation** — ✓ Done. Spawns fresh terminal sessions on workspace transitions with a specialized handshake greeting and origin-path propagation via `--teleported-from`. New window matches the originating window’s pixel size and position; launches without splash screen. Source terminal auto-closes via a background watcher on the parent `cmd.exe` (Windows Terminal excluded). Sovereign OS directories (Desktop, Downloads, Documents, Pictures, Videos, Music) redirect all runtime state to `~/.hematite/` — no `.hematite/` folders created in those locations. Bare name support: `/cd downloads`, `/cd desktop`, `/cd ~` all resolve without `@` prefix.
- **Native Tool Mandate** — ✓ Done. Triage hierarchy and system prompts now strictly prioritize native surgical tools over MCP mutations for local filesystem operations, enforced by `surgical_filesystem_mode` in `routing.rs`.
- **Deep WSL / Docker filesystem auditing** — ✓ Done. Shipped as `inspect_host(topic: “docker_filesystems”)` and `inspect_host(topic: “wsl_filesystems”)`. Covers bind mounts, named volumes, Docker Desktop disk-image growth, WSL rootfs usage, host-side `ext4.vhdx` sizing, and `/mnt/c` bridge checks, with output shaped as `finding -> impact -> exact fix steps`.
- **Advanced LAN / UPnP / neighborhood inspection** — ✓ Done. Shipped as `inspect_host(topic: “lan_discovery”)`. Covers neighborhood discovery summary, SMB/NetBIOS visibility, mDNS/SSDP/UPnP listener surface, gateway/device-discovery hints, and plain-English diagnosis for “discovery broken vs service missing vs firewall blocked”.
- **Voltage telemetry for overclocker** — ✓ Done. `overclocker` now reports real board-power context plus explicit GPU-voltage availability on the active NVIDIA driver path, and only shows CPU voltage when WMI exposes a decodable firmware-reported value. The wording stays strict: power draw is not presented as voltage telemetry.
- **Audio + microphone troubleshooting** — ✓ Done. Shipped as `inspect_host(topic: “audio”)`. Covers Windows Audio service health, playback and recording endpoint inventory, microphone and speaker path checks, Bluetooth-audio crossover, and plain-English diagnosis for “no sound / bad mic / crackling”.
- **Bluetooth troubleshooting** — ✓ Done. Shipped as `inspect_host(topic: “bluetooth”)`. Covers Bluetooth radio presence, service health, paired-device inventory, Bluetooth audio endpoint crossover, and plain-English diagnosis for “won’t pair / keeps disconnecting / wrong headset role”.
- **Camera + privacy-permission auditing** — ✓ Done. Shipped as `inspect_host(topic: “camera”)`. Covers PnP camera/webcam device inventory, Windows camera privacy registry state, Windows Hello biometric camera detection, and plain-English diagnosis for “camera not working / blocked by privacy settings”.
- **Windows Hello / sign-in recovery** — ✓ Done. Shipped as `inspect_host(topic: “sign_in”)`. Covers Windows Hello and biometric service state (WBioSrvc), recent logon failure events (EventID 4625), enrolled credential providers, and plain-English diagnosis for “PIN/fingerprint not working / can’t sign in”.
- **Search indexing diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “search_index”)`. Covers Windows Search (WSearch) service state, indexer registry configuration, indexed locations, recent indexer errors, and plain-English diagnosis for “search not finding files / indexer stopped”.
- **Display configuration** — ✓ Done. Shipped as `inspect_host(topic: “display_config”)`. Covers active monitor resolution, refresh rate, bits-per-pixel, video adapter driver version, connected monitor PnP names, and DPI/scaling percentage via Win32 GDI.
- **NTP / time sync** — ✓ Done. Shipped as `inspect_host(topic: “ntp”)`. Covers Windows Time service (W32Time) health, NTP source and last sync via w32tm, configured NTP peers (registry fallback), and plain-English diagnosis for clock drift or sync failure.
- **CPU power and frequency** — ✓ Done. Shipped as `inspect_host(topic: “cpu_power”)`. Covers active power plan, processor min/max state and turbo boost mode, current CPU clock and load via WMI Win32_Processor, thermal zone temperatures, and diagnosis for “CPU stuck slow / boost disabled / power plan capping frequency”.
- **Credential Manager diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “credentials”)`. Covers vault summary, credential target inventory, type counts, and hygiene warnings without exposing secret values.
- **TPM / Secure Boot diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “tpm”)`. Covers TPM presence/readiness/spec version, Secure Boot state, firmware mode, and plain-English diagnosis for Windows 11 or BitLocker security posture.
- **Browser health diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “browser_health”)`. Covers Edge/Chrome/Firefox inventory, default browser and protocol associations, WebView2 runtime health, browser proxy/policy overrides, profile/cache pressure, and recent browser crash evidence.
- **Microsoft 365 identity-auth diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “identity_auth”)`. Covers TokenBroker / WAM / AAD Broker Plugin state, `dsregcmd` device-registration signals, Office/Teams/OneDrive account mismatch detection, WebView2 auth dependency state, and recent auth-related events.
- **Outlook diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “outlook”)`. Covers classic Outlook and new Outlook for Windows install inventory, running process state and RAM usage, mail profile count, OST and PST file discovery with sizes, add-in inventory with load behavior and resiliency-disabled items, authentication and token broker cache state, and recent Outlook crash evidence from the Application event log.
- **Teams diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “teams”)`. Covers classic Teams and new Teams (MSTeams MSIX) install inventory, running process state and RAM usage, cache directory sizing for both classic and new Teams, WebView2 runtime dependency check, account and sign-in state, audio/video device binding, and recent Teams crash evidence from the Application event log.
- **Windows backup diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “windows_backup”)`. Covers File History service state and last backup date/target drive, Windows Backup (wbadmin) last successful backup versions and scheduled tasks, System Restore enabled state and most recent restore point, OneDrive Known Folder Move per-account protection state, and recent backup failure events from the Application event log.
- **Hyper-V diagnostics** — ✓ Done. Shipped as `inspect_host(topic: “hyperv”)`. Covers Hyper-V role state (VMMS service, feature installed), VM inventory with name, state, CPU%, RAM, and uptime, VM network switch inventory (External/Internal/Private with bound NIC), VM checkpoint listing with creation timestamps, and host RAM overcommit detection. Reports gracefully if Hyper-V is not installed.
- **Application crash triage** — ✓ Done. Shipped as `inspect_host(topic: “app_crashes”)`. Faulting application name/version, faulting module, exception code, crash vs hang classification, WER archive count, crash frequency over 7 days. Accepts optional `process` arg to filter by app name. Distinct from `recent_crashes` (BSOD/kernel events); routing detects natural-language variants including plural/verb forms.
- **MCP server mode** — ✓ Done. `hematite --mcp-server` starts a JSON-RPC 2.0 newline-delimited stdio server exposing all 116+ `inspect_host` topics to any MCP-capable client (Claude Desktop, OpenClaw, Cursor, Windsurf) with no TUI, no local model required. Implemented in `src/agent/mcp_server.rs`.
- **Edge redaction Tier 1** — ✓ Done. `--edge-redact` applies compiled regex patterns post-inspect_host: strips usernames in paths, MAC addresses, serial numbers, hostnames, AWS key IDs, and credential-shaped env values. Each response includes a machine-readable receipt header with per-category counts. Implemented in `src/agent/edge_redact.rs`.
- **Semantic redaction Tier 2 + privacy gateway** — ✓ Done. `--semantic-redact` routes raw inspect_host output through the local LM Studio model with a hardened privacy prompt before any data leaves the machine. Fail-safe: unreachable model returns an error, never raw data. Jailbreak resistance: `<diagnostic_data>` delimiters, refusal detection, unknown MCP args stripped. Tier 1 runs after as safety net. Policy file (`.hematite/redact_policy.json`) provides per-topic block lists, whitelist mode, and redaction level overrides. Metadata-only audit trail written to `~/.hematite/redact_audit.jsonl`. Implemented across `src/agent/semantic_redact.rs`, `src/agent/redact_policy.rs`, `src/agent/redact_audit.rs`.

### Next Up — highest-value missing support lanes

- **Enterprise enrollment diagnostics** — add `inspect_host(topic: “mdm_enrollment”)` for Intune / Autopilot / MDM enrollment state, common ESP blockers, and the first concrete support lane for managed Windows fleets.

### Deferred — implement if users request it

- **Per-workspace model profiles** — let `.hematite/settings.json` specify a preferred model, context ceiling, and embed model per project; useful when different repos need different size/speed tradeoffs. LM Studio makes manual model swaps easy and Hematite detects the active model automatically, so this is low priority until users hit the friction.

- **Whisper voice input** — closes the voice loop (TTS out already ships; this adds STT in). Deferred because Hematite's primary users are keyboard-comfortable developers where typing is faster and more accurate than voice for code-specific terminology. If you want to add it: use `whisper-rs` (Rust bindings to `whisper.cpp`, statically linked — no extra DLLs), `cpal` for audio capture, and embed the `tiny.en` or `base.en` GGUF model via `include_bytes!` at compile time following the same pattern as Kokoro's ONNX model. Wire a `Ctrl+M` hotkey in `tui.rs`, add a recording loop in `src/ui/voice.rs`, and pipe the transcript into the TUI input field. The result is a single self-contained binary with no install requirements — the binary just grows by ~75–150 MB depending on which model you embed. Enable it behind `--features embedded-whisper` so users who don't want the size increase can skip it.

### Tier 2 — Worth doing when local models handle it reliably

- **Workflow engine** — encode multi-step coding workflows (read → edit → verify → commit) as explicit typed state machines that the harness drives, not the model re-plans each turn.
- **Tool dependency graph** — before executing a plan, check whether its tool sequence is valid (no write before read, no verify before edit). Block impossible plans before they waste a turn.
- **Context budget ledger** — track token cost per tool call and per turn; surface a real budget breakdown so the operator can see why a session hit the ceiling, not just that it did.
- **Multi-model routing** — for tasks that need a faster or smaller model (search, classification, label generation), route specific tool calls to a lightweight model while keeping the main session on the primary coding model. The groundwork for this already exists: `--semantic-redact` accepts any `--url` endpoint, so a dedicated compact model (e.g. [Bonsai 8B Q1_0](https://huggingface.co/prism-ml/Bonsai-8B-gguf) at 1.15 GB) can run as the privacy summarizer alongside Qwen3.5 9B + nomic-embed on a single RTX 4070 with VRAM to spare. The next step is exposing a `swarm_url` config key so swarm workers can be dispatched to a separate lightweight endpoint — enabling a local agent web with no cloud required at any layer.

### Tier 3 — Revisit when local 9B models catch frontier capability

- **The Vein as an explicit knowledge base** — manual `remember this` and `forget this` operator commands with durable, typed knowledge entries that survive `/new` and workspace resets.
- **Hardware-aware autonomy** — let the harness self-limit swarm fanout, tool parallelism, and context depth based on live VRAM and context-pressure readings without requiring operator intervention.
- **Privacy audit layer** — before `shell` or `run_code` runs, scan for credential patterns (API keys, tokens, env vars) in arguments and offer a redact-and-confirm path.
- **Session continuity across restarts** — ✓ Done (see Shipped above). Goal, working set, running summary, and last verification result all survive restarts via `.hematite/session.json`.
