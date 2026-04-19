# Hematite Capabilities

This document summarizes the technical strengths of **Hematite-CLI** as a local GPU-aware coding harness for LM Studio and Gemma-family models, with the strongest optimization focus on single-GPU consumer hardware such as the RTX 4070 class.

Hematite is not trying to be a generic cloud-agent platform in a terminal skin. Its product thesis is narrower and stronger:

- be the best **local coding harness for LM Studio**
- be honest about **consumer GPU limits**
- make **runtime truth, recovery, and repo grounding** visible to the operator
- turn open local models into a serious project-work tool instead of a chat wrapper

That is the lens for the capabilities below.

## What Makes It Distinct

- **Local runtime truth**: live model/context sync, prompt-budget pressure, compaction pressure, typed provider states, and recovery recipes are surfaced directly in the operator UI
- **Repo-grounded behavior**: Hematite prefers architecture tracing, repo mapping, tool discipline, and bounded inspection over freeform model improvisation
- **Single-GPU engineering**: context shaping, compaction, fallback prompting, and recovery are built around what a 4070-class machine can actually sustain
- **Windows-first local quality**: PowerShell behavior, path handling, packaging, and terminal ergonomics are treated as first-class product concerns
- **Agent-harness boundary**: LM Studio is the model runtime; Hematite owns the workflow, tooling, TUI, safety, retrieval, and orchestration layer
- **Full OS stack coverage**: 111+ read-only diagnostic topics covering SysAdmin and Network Admin domains.
- **Diagnostic Command Redirection**: Automated redirection of raw diagnostic shell commands to structured `inspect_host` topics to minimize operator prompts.
- **Automated Identity Retrieval**: Proactive SID and group membership lookup for local and active directory users to prevent diagnostic loops.
- **Voice Engine error handling**: Native ONNX synthesis error suppression in `hematite-kokoros` to maintain stream stability.
- **Hardware telemetry integration**: Uses live disk queue depth, VRAM usage, and I/O metrics to inform architectural grounding.
- **Deterministic Workstation Routing**: Hardened intent classification that surgically routes workstation requests to precise filesystem tools, pruning risky shell and workflow paths.
- **Authoritative Path Resolution**: Core-level support for sovereign path tokens and bare directory aliases (`downloads`, `desktop`, `docs`, `pictures`, `videos`, `music`, `home`, `temp`, `~`, `@DESKTOP`, `@DOCUMENTS`, `@MUSIC`, `@VIDEOS`, `@PICTURES`, etc.) using OS-authoritative shell folder hooks.
- **Runtime-state anchoring**: Hematite resolves its runtime-state directory centrally so sovereign OS folders such as Desktop and Downloads fall back to `~/.hematite/` instead of accumulating local workspace artifacts.
- **Heuristic Command Sanitizer**: Mandatory execution gate that blocks natural language injection or conversational "overthinking" from being passed to shell commands.
- **Workspace inspection visibility**: Access to hidden directories (`.hematite`, `.git`) for locating benchmarking targets and runtime artifacts.

## 1. Model-Native Reasoning Flow

Hematite is built to preserve a separation between internal reasoning and user-facing output.

- **Reasoning channel support**: the inference layer parses model-native reasoning markers and keeps them out of the main chat transcript
- **Clean dialogue surface**: internal planning stays in the side panel instead of leaking into the main response
- **Tool-first workflow**: reasoning, tool calls, and final output follow a consistent turn structure

## 2. Precision Editing

Hematite is optimized for controlled code edits on large files.

- **Search-and-replace editing**: `multi_search_replace` requires exact local anchors instead of fragile absolute offsets
- **Failure over corruption**: malformed or weak matches are rejected rather than applied speculatively
- **Multi-hunk support**: disconnected edits can be applied safely in one turn without index drift

## 3. Hardware Awareness

Hematite continuously adapts to the machine it is running on.

- **VRAM monitoring**: live GPU usage is tracked so the harness can react before the session destabilizes
- **Adaptive brief mode**: output and worker behavior can tighten automatically under memory pressure
- **Single-GPU focus**: the runtime is shaped around one practical local GPU, not multi-GPU or cloud assumptions
- **4070-class target**: the design center is the common 12 GB consumer setup where open models need careful context shaping, compaction, and tool discipline
- **Live LM Studio context detection**: startup now prefers the loaded model's `loaded_context_length` from LM Studio so Hematite budgets against the active runtime context instead of an outdated fallback field
- **Live runtime-profile refresh**: before each turn, Hematite can resync the loaded LM Studio model ID and active context budget so model swaps or context changes do not require a full Hematite restart
- **Quiet background runtime sync**: while idle, Hematite can keep the status bar aligned with LM Studio's live model and CTX state and only emits a visible operator message when the runtime profile actually changes
- **Compact LM runtime badge**: the bottom status bar now exposes a low-noise LM Studio state badge so the operator can see live, stale, warning, or context-ceiling conditions at a glance
- **Provider-state machine**: retries and runtime failures emit compact provider states such as recovering, degraded, or context-ceiling so the operator can see what Hematite is doing without parsing long failure prose
- **Failure-state persistence**: a runtime refresh can update model and CTX without immediately clearing a real `LM:CEIL` or `LM:WARN` condition; those states persist until successful output proves recovery
- **Compaction-pressure meter**: the bottom bar now shows a compact percentage badge tied to Hematite's real adaptive compaction threshold so the operator can see when conversation history is approaching summary-chaining pressure
- **Prompt-budget meter**: the operator surface now exposes a separate `BUD:NN%` badge for total turn payload pressure against the live LM Studio context window, which catches small-context prompt blowups that are not visible from history compaction pressure alone
- **Tighter operator footer**: the input/status surface now prioritizes real controls and real signals, including a live session error count, instead of spending width on dead counters or unreliable terminal hints
- **Runtime-owned provider state**: recovery, degraded, live, and context-ceiling transitions are now emitted by the runtime layer itself instead of being guessed by the TUI from rendered tokens or error strings
- **Typed operator checkpoints**: SPECULAR now receives explicit runtime checkpoint states for provider recovery, prompt-budget reduction, history compaction, blocked policy paths, blocked recent-file-evidence edits, blocked exact-line-window edits, and other recovery/blocker transitions
- **Typed recovery recipes**: retries, runtime refreshes, prompt-budget reduction, history compaction, and proof-before-edit recovery are now described by named recovery scenarios and compact step recipes instead of only ad hoc branch logic
- **Runtime bundle boundary**: startup assembly for engine, channels, watcher, voice, swarm, and LM Studio profile sync now lives behind a typed runtime bundle instead of being hand-wired directly in `main.rs`
- **Real-time silicon tracking**: `overclocker` delivers high-fidelity telemetry informed by the **Zero-Overhead Silicon Historian**—a 10-point RAM-only buffer tracking session trends (Temp/Clocks/Power anomalies) without disk baggage.
- **NVIDIA Deep-Sense**: precision GPU telemetry including real-time power draw and power-cap context (W), graphics/memory clocks (MHz), fan curves, and explicit GPU-voltage availability reporting, plus the **Precision Throttle Truth** engine to decode NVIDIA bitmasks into root-cause casualties (Power vs Thermal).
- **Typed permission enforcement**: tool authorization now converges through one runtime decision layer for allow, ask, or deny outcomes instead of splitting shell rules, MCP approval defaults, safe-path bypasses, and shell-risk classification across ad hoc branches
- **Workspace trust state**: the current repo root is resolved through a typed trust policy, so destructive or external actions can behave differently in trusted, unknown, or explicitly denied workspaces
- **Registry-owned tool metadata**: repo reads, repo writes, git tools, verification tools, architecture tools, workflow helpers, research tools, vision tools, and MCP tools now carry explicit runtime metadata so mutability, trust sensitivity, plan fit, and parallel-safe execution are less dependent on ad hoc name lists
- **Dedicated tool registry boundary**: built-in tool definitions and builtin-tool dispatch now live behind `src/agent/tool_registry.rs` so the conversation loop owns less catalog/dispatch glue and more of the actual turn policy
- **Typed MCP lifecycle state**: MCP server availability is now surfaced as unconfigured, healthy, degraded, or failed runtime state so external-server issues do not hide inside tool refresh side effects
- **Intent-class routing**: stable product truth, runtime diagnosis, repository architecture, toolchain guidance, and capability questions now flow through one shared intent classifier instead of a long stack of isolated phrase gates
- **Typed session ledger**: compact carry-over now remembers the latest checkpoint, blocker, recovery step, verification result, and compaction metadata instead of preserving only task text and working-set hints
- **Tiny-context fallback profile**: when LM Studio serves a very small active context window, Hematite can switch to a slimmer system prompt so simple prompts still fit instead of immediately exhausting the budget
- **Manual runtime refresh**: `/runtime-refresh` lets the operator force an LM Studio profile resync on demand, and context-window failures trigger the same refresh path automatically

## 4. SysAdmin and Network Admin

Hematite ships a complete workstation inspection layer that covers the full OS stack in plain English. All topics are read-only — the harness answers from real observed state, not model guesses.

**SysAdmin topics (73+):**

- **Resource load** (`resource_load`) — live CPU and RAM usage with top consumers
- **Processes** (`processes`) — per-process CPU time, memory, [I/O R:N/W:N] operation counts, and PID analytics
- **Services** (`services`) — running Windows services, startup types, and state
- **Ports** (`ports`) — listening TCP/UDP ports with owning process
- **Storage** (`storage`) — all-drives capacity with ASCII bar charts, developer cache sizing, and real-time Disk Intensity (Average Disk Queue Length)
- **Hardware** (`hardware`) — full hardware DNA: CPU model/cores/clock, RAM total/speed/sticks/channel, GPU name/driver/resolution, motherboard/BIOS manufacturer/version, and Virtualization Health (Hypervisor status and SLAT/VT-x capability)
- **Sessions** (`sessions`) — audits active and disconnected user logon sessions with terminal service info
- **Health report** (`health_report`) — tiered plain-English verdict (ALL GOOD / WORTH A LOOK / ACTION REQUIRED) across disk, RAM, tools, and recent error events
- **Windows Update** (`updates`) — last install date, pending update count, Windows Update service state
- **Security** (`security`) — Defender real-time protection, last scan age, signature freshness, firewall profile states, Windows activation, UAC state
- **Pending reboot** (`pending_reboot`) — detects queued restarts from Windows Update, CBS, and file rename operations
- **Disk health** (`disk_health`) — physical drive health via Get-PhysicalDisk and SMART failure prediction
- **Battery** (`battery`) — charge level, status, estimated runtime, wear level; reports gracefully on desktops
- **Crash history** (`recent_crashes`) — BSOD/unexpected shutdown events and application crash/hang events from the Windows event log
- **Scheduled tasks** (`scheduled_tasks`) — all non-disabled scheduled tasks with name, path, last run time, and executable
- **Dev conflicts** (`dev_conflicts`) — cross-tool environment conflict detection: Node version managers, Python 2/3 ambiguity, conda shadowing, Rust toolchain path conflicts, Git identity/signing, duplicate PATH entries
- **Path and toolchains** (`path`) — full PATH inspection with version detection for installed developer tools
- **Log check** (`log_check`) — recent system error events from the Windows event log
- **Startup items** (`startup_items`) — boot-time programs and their startup types
- **OS config** (`os_config`) — firewall profiles, power plan, and uptime
- **User accounts** (`user_accounts`) — local user accounts (name, enabled, last logon, password required), Administrators group members, active logon sessions, and elevated process state; redirected from `Get-LocalUser` and `net user`
- **Active Directory User** (`ad_user`) — precise user/group lookup via Get-ADUser or net user/domain; shows SID, enabled status, password expiry, and group memberships; includes **Self-Aware discovery** for 'Who am I?' queries
- **Audit policy** (`audit_policy`) — Windows audit policy via auditpol; shows which event categories log Success/Failure; flags if no categories are enabled
- **Hyper-V** (`hyperv`) — live inventory of Virtual Machines with name, state, uptime, and CPU/Memory load stats
- **Shares** (`shares`) — SMB shares exposed by this machine (flags custom non-admin shares), SMB security settings (SMB1/SMB2 state, signing, encryption), and mapped network drives
- **BitLocker** (`bitlocker`) — drive encryption state per volume (PROTECTED/UNPROTECTED), protection method, and SMB1 warning; LUKS on Linux
- **RDP** (`rdp`) — Remote Desktop enabled state (registry fDenyTSConnections), port number, NLA/UserAuthentication, firewall group status, and active sessions
- **Shadow copies** (`shadow_copies`) — VSS shadow copy count and storage allocation, System Restore points, and LVM snapshots on Linux
- **Page file** (`pagefile`) — virtual memory allocated/current/peak MB, system-managed vs fixed config, RAM context, and high-usage warning
- **Windows features** (`windows_features`) — enabled optional features with count, notable feature flags (Hyper-V, IIS, Telnet, TFTP, NFS), and quick-check for six key features
- **Printers** (`printers`) — installed printers with default flag, active print jobs; CUPS on Linux
- **WinRM** (`winrm`) — Windows Remote Management service state, listener config, TrustedHosts, and Test-WSMan connectivity check
- **Device health** (`device_health`) — precision detection of malfunctioning hardware via PnP ConfigManager error codes (the "Yellow Bang" devices in Device Manager)
- **Drivers** (`drivers`) — comprehensive audit of active system drivers with name, type, path, and operational state
- **Peripherals** (`peripherals`) — deep-dive into USB controllers, HID devices (keyboard/mouse class), and connected monitors
- **Audio** (`audio`) — Windows Audio service health, playback/recording endpoint inventory, microphone/speaker path checks, Bluetooth-audio crossover, and plain-English diagnosis for “no sound / bad mic / crackling”
- **Bluetooth** (`bluetooth`) — Bluetooth radio state, service health, paired-device inventory, headset/audio endpoint crossover, and plain-English diagnosis for “won’t pair / keeps disconnecting / wrong headset role”
- **Camera** (`camera`) — PnP camera/webcam device inventory, Windows camera privacy registry state, Windows Hello biometric camera detection, and plain-English diagnosis for “camera not working / blocked by privacy settings”
- **Sign-In / Windows Hello** (`sign_in`) — Windows Hello and biometric service state, WBioSrvc health, recent logon failure events (EventID 4625), enrolled credential providers, and plain-English diagnosis for “PIN/fingerprint not working / can’t sign in”
- **Installer Health** (`installer_health`) — Windows Installer (`msiserver`), AppX/Store install services, `winget`/Desktop App Installer presence, Microsoft Store package health, pending reboot or in-progress installer blockers, and recent MSI/AppX failure evidence
- **OneDrive** (`onedrive`) — client install/running state, configured accounts, sync-root existence, OneDrive policy blockers, and Known Folder Backup/Desktop/Documents/Pictures redirection state
- **Browser Health** (`browser_health`) — browser inventory and versions for Edge/Chrome/Firefox, default browser/protocol associations, runtime process and working-set pressure, WebView2 runtime health, browser proxy/policy overrides, profile/cache pressure, and recent browser crash evidence
- **Outlook** (`outlook`) — classic Outlook and new Outlook for Windows install inventory, running process state and RAM usage, mail profile count, OST and PST file discovery with sizes, add-in inventory with load behavior and resiliency-disabled items, authentication and token broker cache state, and recent Outlook crash evidence from the Application event log
- **Search Index** (`search_index`) — Windows Search (WSearch) service state, indexer registry configuration, indexed locations (shell namespaces + registry fallback), recent indexer errors, and plain-English diagnosis for “search not finding files / indexer stopped”
- **Display Config** (`display_config`) — active monitor resolution, refresh rate, bits-per-pixel, video adapter driver version, connected monitor names/PnP IDs, and DPI/scaling percentage via Win32 GDI
- **NTP / Time Sync** (`ntp`) — Windows Time service (W32Time) health, NTP source and last sync via w32tm, configured NTP peers/registry, and plain-English diagnosis for clock drift or sync failure
- **CPU Power** (`cpu_power`) — active power plan, processor min/max state and turbo boost mode, current CPU clock and load via WMI Win32_Processor, thermal zone temperatures, and diagnosis for “CPU stuck slow / boost disabled / power plan capping frequency”
- **Credentials** (`credentials`) — Windows Credential Manager vault summary, credential target inventory via cmdkey, type counts, and plain-English hygiene warnings without ever exposing secret values
- **TPM / Secure Boot** (`tpm`) — TPM presence/readiness/spec version, Secure Boot state, firmware mode (UEFI vs legacy BIOS), and plain-English diagnosis for Windows 11 or BitLocker security posture
- **Group Policy** (`gpo`) — applied Group Policy Objects (computer scope), filtering status; requires Administrator elevation on Windows
- **Certificates** (`certificates`) — local personal certificates with subject, thumbprint, expiry date; flags certs expiring within 30 days
- **Integrity** (`integrity`) — Windows component store health via SFC/DISM registry and log visibility; flags Corrupt or AutoRepairNeeded state
- **Domain Context** (`domain`) — Active Directory and domain join status: Join Status (DOMAIN/WORKGROUP), Domain name, and NetBIOS name
- **Permissions** (`permissions`) — Precision NTFS/ACL security audits; identifies non-admin write access and inheritance state
- **Login History** (`login_history`) — Triage of recent successful and failed logon events from the security log (Event ID 4624)
- **Registry Audit** (`registry_audit`) — Proactive security audit for persistence hijacks: IFEO debuggers, Winlogon Shell overrides, BootExecute, and Sticky Keys exploits
- **Thermal Health** (`thermal`) — Precision telemetry for CPU temperature, thermal margins, and active throttling indicators
- **Windows Activation** (`activation`) — Audits Windows license status, genuine status, and Product ID/Key metadata
- **Patch History** (`patch_history`) — Windows HotFix and KB update audit (last 48h focus)
- **Repo Doctor** (`repo_doctor`) — Workspaces health audit: git status, uncommitted changes, and build-file presence
- **Disk Benchmark** (`disk_benchmark`) — Sequential read/write throughput and latency measurements on the workspace drive
- **Overclocker Telemetry** (`overclocker`) — Precision real-time silicon performance: NVIDIA graphics/memory clocks, fan speeds, board power draw and power-cap context (W), explicit GPU-voltage availability reporting, firmware-reported CPU voltage when WMI exposes it, root-cause throttle decoding (Power vs Thermal), and **Session History** (in-memory trends/anomalies) identifying hardware drift since startup.
- **Authoritative Directory Audit** (`directory`, `desktop`, `downloads`, `music`, `videos`, `pictures`) — High-precision directory listing using OS-level tokens; instantly routes to surgical tools via the deterministic intent engine.
- **Share Access** (`share_access`) — Connectivity and readability test for network shares and UNC paths.

**Network Admin topics (28+):**

- **Latency** (`latency`) — ping RTT (min/avg/max) and packet loss to the default gateway, Cloudflare DNS (1.1.1.1), and Google DNS (8.8.8.8); findings for unreachable targets, high packet loss, and elevated latency
- **Network Adapter** (`network_adapter`) — NIC inventory (link speed, MAC, driver version), offload settings (LSO/RSS/TCP checksum offload/jumbo frames) per adapter, error and discard counters, and wake-on-LAN / power management state
- **DHCP Lease** (`dhcp`) — DHCP lease details per adapter: server IP, lease obtained, lease expires, subnet mask, DNS servers; findings for expired or imminently-expiring leases
- **MTU** (`mtu`) — per-adapter IPv4/IPv6 MTU via `Get-NetIPInterface`; path MTU discovery test to 8.8.8.8 using DF-bit pings at 1472/1400/1280/576 bytes; findings for restricted MTU, VPN fragmentation, or blocked ICMP
- **IPv6** (`ipv6`) — per-adapter IPv6 addresses (global/link-local/ULA) with prefix origin (SLAAC/DHCPv6/static), IPv6 default gateway, DHCPv6 lease assignments, privacy extension state (RFC 4941), and tunnel adapter inventory (Teredo/6to4/ISATAP); findings for no global address or missing gateway
- **TCP Parameters** (`tcp_params`) — TCP autotuning level, congestion provider (CUBIC/NewReno), initial congestion window, scaling heuristics, dynamic port range, chimney offload state, and ECN capability; findings for disabled autotuning or non-standard congestion provider
- **WLAN Profiles** (`wlan_profiles`) — saved wireless profiles with authentication type (WPA2/WPA3/WEP/Open), cipher, connection mode, and auto-connect state; currently connected SSID, BSSID, signal, and radio type; findings for profiles using weak/open authentication
- **IPSec** (`ipsec`) — enabled IPSec connection security rules with mode and action; active main-mode and quick-mode SAs with local/remote address pairs; IKE Policy Agent service state; findings for active tunnels
- **Connectivity** (`connectivity`) — internet reachability test (DNS + ICMP + HTTPS) with latency and failure diagnosis
- **Wi-Fi** (`wifi`) — connected SSID, signal strength, channel, frequency band, and adapter details
- **Active connections** (`connections`) — all established and listening TCP/UDP connections with owning process
- **VPN** (`vpn`) — VPN adapter detection, state, and assigned IP address
- **Proxy** (`proxy`) — system-level proxy settings (WinHTTP / per-user / environment variables)
- **Firewall rules** (`firewall_rules`) — active Windows Firewall rules allowing inbound traffic
- **Traceroute** (`traceroute`) — hop-by-hop path to a target with round-trip times and latency spikes
- **DNS cache** (`dns_cache`) — current local DNS resolver cache entries
- **ARP table** (`arp`) — local ARP cache mapping IP addresses to MAC addresses
- **Routing table** (`route_table`) — full IP routing table with interface, next-hop, and metric
- **LAN discovery** (`lan_discovery`) — neighborhood, NetBIOS/SMB visibility, mDNS/SSDP/UPnP listener surface, gateway hints, and plain-English diagnosis for “can’t see that NAS/printer/PC”
- **Network stats** (`network_stats`) — per-adapter RX/TX throughput (MB), error counts, drop counts, link speed, and duplex; flags adapters with errors or drops
- **UDP ports** (`udp_ports`) — active UDP listeners with owning process name and annotations for well-known ports (DNS, NTP, NetBIOS, mDNS, SSDP, IKE, SNMP)
- **DNS Lookup** (`dns_lookup`) — specific high-precision DNS query for A/AAAA, MX, TXT, SRV, and other record types; now handles plain-English domain-to-IP questions and defaults to `A` when the user asks for a hostname/IP answer without naming a record type
- **IP Configuration** (`ip_config`) — full adapter detail (ipconfig /all equivalent); surfaces DHCP server, lease times, and multi-IP interfaces
- **NetBIOS** (`netbios`) — NetBIOS over TCP/IP state per adapter (enabled/disabled/DHCP), WINS server configuration, nbtstat registered names, and active NetBIOS sessions; flags enabled NetBIOS as a potential attack surface
- **NIC Teaming** (`nic_teaming`) — LBFO team inventory (mode, load-balancing algorithm, status, link speed), team member detail and operational state; flags degraded teams or inactive members
- **SNMP** (`snmp`) — Windows SNMP agent service state, community string presence audit (values redacted), permitted manager list, SNMP Trap service; flags running agents and the well-known 'public' community string as a risk
- **Port Test** (`port_test`) — TCP port reachability test to any remote host and port via `Test-NetConnection`; returns OPEN/CLOSED/FILTERED with ICMP ping result, source address, and interface used. Use args `host` and `port`.
- **Network Profile** (`network_profile`) — Windows network location profile per interface (Public/Private/DomainAuthenticated), IPv4/IPv6 connectivity state; flags Public-category interfaces and domain-authenticated connections

**Intent-based diagnostic orchestration:**

When a user asks about multiple inspection topics or uses common trouble keywords like "slow", "lag", or "I/O pressure", Hematite automatically detects all matching topics before the model turn and runs all `inspect_host` calls automatically. The combined results are injected as a `loop_intervention` so the model synthesizes from real data instead of orchestrating tool calls one by one. This eliminates redundant round-trips and prevents the model from collapsing multi-topic requests into a single generic topic.

**Shell auto-redirect:**

When the model calls `shell` with a command that matches a structured host inspection topic (e.g. `arp -a`, `tracert`, `Get-DnsClientCache`, `Get-NetRoute`, `Get-Process`), the harness silently redirects it to the correct `inspect_host` topic.

**Redirection discipline:**
 
Hematite implements a definitive loop-breaker for auto-redirected shell calls. If the model attempts to call `shell` repeatedly for the same diagnostic intent, the harness provides a short "Action Handled" message instead of flooding the context with redundant telemetry. The **Synchronized Enforcer** ensures that shell diagnostics are only blocked if a native topic is actually available to take over.

**Developer tooling topics (10):**

- **Environment variables** (`env`) — total count, developer/tool vars (CARGO_HOME, JAVA_HOME, GOPATH, VIRTUAL_ENV, DOCKER_HOST, etc.), secret-shaped vars shown as `[SET, N chars]` only — values never exposed; PATH entry count with pointer to the path topic
- **Hosts file** (`hosts_file`) — reads `/etc/hosts` (Windows: `drivers\etc\hosts`); shows active entries, flags custom non-loopback entries, includes full file content
- **Docker** (`docker`) — Docker Engine version, daemon health, running containers with status and ports, local images, Docker Compose projects, active context; reports gracefully if not installed or daemon is down
- **WSL** (`wsl`) — Windows Subsystem for Linux distros with state (Running/Stopped), WSL version metadata; Windows-only feature, reports platform note on Linux/macOS
- **SSH** (`ssh`) — SSH client version, sshd service state, `~/.ssh` inventory (known_hosts entry count, authorized_keys count, private key files present), `~/.ssh/config` host entries with hostname/user/port/identity details
- **Installed software** (`installed_software`) — winget list on Windows (registry scan fallback); dpkg/rpm/pacman on Linux; brew + mas on macOS; paginated with max_entries
- **Git config** (`git_config`) — global git config grouped by Identity, Core, Commit/Signing, Push/Pull, Credential, Branch sections; local repo config; git aliases; points at missing config if not set up
- **Databases** (`databases`) — detects running local database engines: PostgreSQL, MySQL/MariaDB, MongoDB, Redis, SQLite, SQL Server, CouchDB, Cassandra, Elasticsearch — via CLI version check, TCP port probe, and OS service state; no credentials required

Additional deep-audit developer topics:

- `docker_filesystems` audits bind mounts, named volumes, per-container mount summaries, and Docker Desktop disk-image growth with `finding -> impact -> fix` output
- `wsl_filesystems` audits WSL distro rootfs usage, host-side `ext4.vhdx` growth, and `/mnt/c` bridge health without starting stopped distros

**Safe remediation:**

`resolve_host_issue` provides a bounded, user-gated path for three fix actions: `install_package` (winget), `restart_service`, and `clear_temp`. Read-only inspection is always available without approval; write actions go through the safe remediation path.

## 5. Workspace-Native Tooling

Hematite is more than a chat shell around a local model.

- **File and shell tools**: direct project reading, editing, search, and shell execution
- **PageRank-powered Repo Maps**: Native context injection leverages `tree-sitter` for AST indexing and `petgraph` PageRank to surface the most structurally important files first — the model wakes up already knowing the architecture without burning tool calls
- **Git-aware workflows**: worktrees, commit helpers, and rollback via hidden ghost snapshots
- **Configurable verification**: `verify_build` can now use per-project build, test, lint, and fix profiles from `.hematite/settings.json` instead of relying only on stack autodetection
- **Project retrieval**: SQLite FTS-backed memory helps recover relevant local context each turn
- **Built-in web research**: `research_web` and `fetch_docs` let the harness search for technical information and pull external docs into a readable form when local context is insufficient
- **Grounded architecture tracing**: `trace_runtime_flow` gives the model a verified read-only path for exact runtime/control-flow questions instead of encouraging confident guessing
- **Grounded architecture overviews**: broad read-only architecture questions now combine the AST injection with one authoritative `trace_runtime_flow` topic instead of drifting into long repo rewrites
- **Grounded toolchain guidance**: `describe_toolchain` gives the model a verified read-only map of Hematite's actual built-in tools, when to use them, and what investigation order makes sense
- **Vision support**: screenshot and diagram analysis can flow through `vision_analyze` when a task benefits from visual inspection

## 6. Stateful Local Workflow

Hematite is built for repeated project use, not one-off prompts.

- **Lightweight session handoff**: Hematite carries forward compact task/project signal instead of replaying full chat residue by default
- **Architect -> code handoff**: `/architect` can persist a compact implementation brief in `.hematite/PLAN.md` and session memory so `/code` can resume from a structured plan
- **Safe Gemma 4 native layer**: Gemma 4 runs get narrow argument normalization for malformed tool calls without changing Hematite's broader conversation protocol
- **Gemma numeric-arg hygiene**: float-shaped tool arguments like `limit: 50.0` or `context: 5.0` are normalized so bounded inspections stay bounded
- **Opt-in Gemma native formatting**: `.hematite/settings.json` can enable Gemma-native request shaping for Gemma 4 models without changing the default path for other models
- **Provider-side prompt preflight**: oversized requests can be blocked before they go to LM Studio, reducing silent near-ceiling hangs
- **Structured runtime failures**: degraded provider turns, context-window overruns, blocked tool calls, and repeated tool loops are surfaced as classified operator states instead of ad hoc error prose
- **One-shot provider recovery**: empty or degraded LM Studio turns get one automatic retry before Hematite escalates the structured failure
- **Streaming-path failure discipline**: plain text generations and startup flows now surface structured provider failures instead of raw stream errors or silent empty completions
- **LM Studio context-mismatch detection**: provider errors like `n_keep >= n_ctx` are classified as `context_window` failures so Hematite points at the real budget mismatch instead of mislabeling them as generic provider degradation
- **Budgeted recursive summaries**: compaction summaries are normalized, deduplicated, and clamped to a real line/character budget so recursive context carry-forward stays cheaper and more stable on small local contexts
- **Session persistence**: active state is saved under the resolved runtime-state directory (`.hematite/` for normal project workspaces, `~/.hematite/` for sovereign OS directories)
- **Task awareness**: local task and planning files can shape agent behavior
- **Instruction discovery**: project rules are loaded automatically from workspace instruction files
- **Sticky workflow modes**: `/ask`, `/code`, `/architect`, `/read-only`, `/teach`, and `/auto` let the operator choose between analysis, implementation, plan-first, hard read-only, and grounded walkthrough behavior
- **Ultra-Deterministic Teleportation**: Seamlessly transition between folders. When moving to a new workspace, Hematite spawns a fresh, pre-navigated terminal, preserves the source window's size and position, skips the splash screen, and gracefully closes the original shell or tab to maintain workstation hygiene. New sessions include a **Teleportation Handshake** greeting that confirms the origin context and transition reasoning.
- **Teacher mode** (`/teach`) — inspects real machine state first via `inspect_host`, then delivers a numbered step-by-step walkthrough for any admin/config/system task; never executes write operations itself; covers driver installs, Group Policy, firewall rules, SSH key generation, WSL setup, service config, Windows activation, registry edits, scheduled tasks, and disk cleanup

- **Grounded storage walkthroughs**: teacher mode can now build step-by-step remediation around `docker_filesystems` and `wsl_filesystems`, so mount/path/storage fixes start from observed bind mounts, VHDX growth, and bridge health rather than generic advice

## 7. Voice and TUI Integration

Hematite includes built-in operator experience features that are part of the product, not bolted on later.

- **Integrated TUI**: dedicated chat, reasoning, status, and input surfaces
- **Self-contained TTS**: Kokoro voice engine (311 MB model, 54 voices, ONNX Runtime 1.24.2) is statically linked into the binary — no install, no Python, no system DLL dependency; `Ctrl+T` to toggle, `/voice` to switch voices, speed/volume configurable in `settings.json`
- **Live diagnostics**: runtime state, GPU load, and tool activity are surfaced during use
- **Hybrid thinking**: non-Gemma models (Qwen etc.) automatically use `/think` mode so the model decides how much reasoning each turn needs without user intervention

## 8. Sandboxed Code Execution

Hematite can run code the model writes in a restricted subprocess — enabling real computation, not pattern-matched guesses from training data.

**Why this matters vs. LM Studio's built-in chat:** LM Studio's chat interface can discuss algorithms, write code snippets, and explain how Fibonacci works. It cannot run any of it. When you ask a local model "what's Fibonacci(20)?", it reaches into training data and gives you a plausible answer — which may be right, may be slightly wrong, and cannot be verified without running it yourself. Hematite closes that gap: the model writes the code, Hematite executes it in a zero-trust sandbox, and the real output comes back in the same turn.

**Proof of concept — SHA-256 hash via Web Crypto API:**

```
User: compute the SHA-256 hash of the string "Hematite"

→ run_code (javascript, Deno sandbox, crypto.subtle.digest)

94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c
```

That result cannot come from training data. SHA-256 is deterministic but not memorizable — no model can produce `94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c` without actually running a hash function. It is real cryptographic computation in a sandboxed Deno process, returned in one tool call. LM Studio's chat UI — regardless of which model is loaded — cannot do this.

- **`run_code` tool**: model writes JavaScript/TypeScript or Python, Hematite executes it and returns the actual output
- **Deno sandbox (JS/TS)**: `--deny-net`, `--deny-env`, `--deny-sys`, `--deny-run`, `--deny-ffi`, `--allow-read/write=.` — zero-trust permission model; no network, no filesystem escape, no native library calls
- **Python sandbox**: blocked socket, subprocess, and dangerous module imports; clean environment via `env_clear`
- **Hard timeout**: 10 seconds by default, model-configurable up to 60 seconds; process killed on expiry
- **Automatic Deno detection**: Hematite finds Deno automatically — checks `settings.json` override, `~/.deno/bin/`, WinGet package store, system PATH, then LM Studio's bundled copy as a last resort. If you have LM Studio installed, you likely already have Deno and JS/TS execution works out of the box with no extra setup
- **Real math and logic**: the model can verify algorithms, run calculations, test data transformations, and fix errors from actual output — not training-data approximations
- **Practical use cases**: check a sorting algorithm on a real dataset, verify a regex against real strings, compute checksums, generate test fixtures, run a quick proof — all without leaving the conversation
- **Automatic computation routing**: Hematite detects when a query requires precise numeric results (hashes, financial math, statistics, date arithmetic, unit conversions, algorithmic checks) and automatically nudges the model to use `run_code` instead of guessing from training data. If the model tries to use `shell` for execution, the harness blocks it and forces a `run_code` retry. If the model writes Python without specifying `language: "python"` and Deno rejects the syntax, the harness catches the parse error and forces a corrective retry with the correct language — no manual intervention required.

## 9. MCP Interoperability

Hematite can extend itself through external MCP servers without making MCP the core identity of the product.

- **Workspace and global MCP config**: discovers `mcp_servers.json` in both scopes
- **Windows launcher compatibility**: resolves `npx`, `.cmd`, and `.bat` wrappers correctly
- **Protocol resilience**: supports newline-delimited stdio and falls back to `Content-Length` framing
- **TUI-safe process handling**: MCP stderr is captured in memory so child processes do not corrupt the terminal UI

## 10. Local-First Product Boundary

Hematite is the **agent harness**. LM Studio is the **model runtime**.

That boundary gives Hematite three advantages:

- model swapping stays easy
- the harness stays focused on workflow quality
- local deployment remains simple for normal users

---

Hematite is strongest when treated as a complete local AI workstation partner: a polished coding harness, a grounded SysAdmin and Network Admin, and a natural-language terminal interface — all GPU-aware, terminal-native, tool-rich, and tuned for serious work on single-GPU consumer hardware, especially RTX 4070-class machines.
