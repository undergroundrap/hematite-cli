# Hematite Prompt Library

A collection of prompts designed to get the most out of Hematite's native capabilities. Copy and paste these directly into the input field.

---

## Workstation Health and Diagnostics

**Full workstation audit**
> "Audit my workstation: check for malfunctioning hardware (Yellow Bangs), show me the BIOS and virtualization DNA, and run a disk benchmark on the current binary."

**Environment doctor**
> "Run an environment doctor on this machine. I want to see PATH health, package manager states, and any dev environment conflicts like Node or Python version ambiguity."

**Device and driver triage**
> "Audit my workstation: check for malfunctioning PnP devices, list active kernel drivers, and show me everything set to run at startup."

**Disk benchmark**
> "Run an I/O intensity report on the binary in `target/release`. Report the average and max disk queue depth and give me a verdict: LIGHT / MODERATE / CRITICAL."

**System health report**
> "Give me a health report on this machine — disk, RAM, tools, and recent error events. Flag anything that needs action."

**Security posture**
> "Check my security posture: Defender state, last scan age, firewall profiles, Windows activation, and UAC level."

**Silicon Deep-Sense (v0.5.6)**
> "How's my silicon health looking? I want to see real-time GPU clocks, power draw, fan speed, and high-fidelity CPU frequency averages."
> *(Triggers Zero-Shot Redirection to: overclocker)*

**Crash and reboot history**
> "Show me any BSOD or unexpected shutdown events from the last week, and tell me if a reboot is currently pending and why."

---

## Network Admin

**Connectivity triage**
> "Check my internet connectivity, Wi-Fi signal strength, and VPN status. If I'm on a VPN, tell me which adapter is handling the tunnel."

**Network map**
> "Show me my routing table, ARP table, and DNS cache. Map out the devices this machine is currently aware of on the local network."

**Open ports and active connections**
> "Show me all listening TCP and UDP ports with their owning processes, and list any established outbound connections."

**DNS and proxy audit**
> "Show me my configured DNS nameservers per adapter and any system proxy settings — WinHTTP, Internet Options, and environment variables."

**Plain-English DNS lookup**
> "What is the IP address of google.com"

**A record lookup**
> "Show me the A record for github.com"

**Browser health**
> "Check browser health on this machine. Tell me if Chrome, Edge, Firefox, or WebView2 look broken, bloated, or policy-constrained."

**Browser policy or proxy interference**
> "Check whether browser policy or proxy settings are interfering with web apps."

**Outlook health**
> "Check Outlook health on this machine."

**Outlook slowness or crash triage**
> "Why is Outlook so slow or broken?"

**Outlook profiles, OST/PST, and add-in audit**
> "Audit Outlook profiles, OST/PST files, and add-in pressure."

**Teams health**
> "Check Teams health on this machine."

**Teams slowness or crash triage**
> "Why is Microsoft Teams so slow or broken?"

**Teams cache and device audit**
> "Audit Teams cache size, WebView2 dependency, and audio/video device binding."

**Windows backup health**
> "Is this machine being backed up?"

**Windows backup status and restore points**
> "Check Windows backup health — File History, wbadmin last backup, and System Restore status."

**Full backup posture audit**
> "Show me my File History configuration, last wbadmin backup, available System Restore points, and whether OneDrive Known Folder Move is protecting my Desktop and Documents."

**Hyper-V VM inventory**
> "List all virtual machines on this machine with their state, RAM, and CPU usage."

**Hyper-V health check**
> "Check Hyper-V health — VM states, network switches, and any checkpoints I should clean up."

**Hyper-V RAM pressure**
> "How much RAM are my VMs using compared to my host machine's physical memory?"

**Firewall rules**
> "List all active inbound firewall rules that allow traffic. Flag anything that looks non-default."

**Traceroute**
> "Trace the network path to 8.8.8.8 and tell me where the latency spikes are."

**Latency and reachability**
> "Check ping RTT and packet loss to my gateway, Cloudflare, and Google DNS. Flag anything unreachable or high latency."

**NIC diagnostics**
> "Audit my network adapter settings — link speed, offload capabilities (LSO/RSS/checksum), error counters, and wake-on-LAN state."

**DHCP lease**
> "Show me my DHCP lease details — which server assigned my IP, when does it expire, and what DNS servers did it hand me?"

**MTU and fragmentation**
> "Check my MTU settings per adapter and run a path MTU discovery test to 8.8.8.8. I think VPN fragmentation is causing issues."

**IPv6 status**
> "Show my IPv6 addresses — am I using SLAAC or DHCPv6? Check for a global unicast address and tell me if privacy extensions are enabled."

**TCP tuning**
> "Show TCP autotuning level, congestion provider, and ECN state on this machine."

**Saved WiFi profiles audit**
> "List all saved wireless profiles with their authentication type. Flag anything using WEP or open auth."

**IPSec tunnel state**
> "Check for active IPSec security associations and IKE tunnel state. Is the Policy Agent service running?"

---

## Developer Tooling

**Toolchain inventory**
> "Show me all installed developer tools with versions — Rust, Node, Python, Go, Docker, Git, and anything else you detect."

**Git config audit**
> "Audit my global git config: identity, signing, push/pull defaults, credential helper, and any local repo overrides."

**Docker state**
> "Show me my Docker daemon state, running containers, local images, and any active Compose projects."

**Database detection**
> "Detect any running local database engines on this machine — PostgreSQL, MySQL, MongoDB, Redis, SQLite, SQL Server, and others."

**SSH inventory**
> "Show me my SSH client version, sshd state, what's in `~/.ssh`, and any host entries in `~/.ssh/config`."

**Dev environment conflicts**
> "Check this machine for cross-tool environment conflicts: Node version managers, Python 2/3 ambiguity, conda shadowing, Rust toolchain path issues, and duplicate PATH entries."

---

## Sandboxed Code Execution

**Verify a calculation**
> "Compute the compound interest on $10,000 at 4.5% annual rate compounded monthly for 3 years. Use `run_code` — do not guess."

**SHA-256 hash**
> "Compute the SHA-256 hash of the string 'Hematite' using the Web Crypto API."

**Date arithmetic**
> "How many days are between 2024-03-15 and 2026-09-01? Run the code and give me the exact number."

**Regex verification**
> "Test this regex against these five strings and tell me which ones match: [your regex here]. Run the actual code."

**Algorithm check**
> "Write and run a quicksort implementation in JavaScript, sort this array: [5, 2, 8, 1, 9, 3], and return the sorted result."

**Hardware-aware implementation**
> "Check the virtualization DNA of this machine before generating this sandbox-heavy execution logic. Make sure the execution timeout reflects the current system load."

---

## Coding and Repo Work

**Read-only architecture trace**
> "Read-only mode. Trace exactly how a user message travels from TUI input to the model and back. Name real channels, functions, and file references — do not guess."

**Targeted implementation**
> "Read-only mode first. Inspect `src/agent/conversation.rs` around the tool dispatch loop, then propose the minimal change needed to [describe goal]."

**Proof before edit**
> "/code Read `src/ui/tui.rs` lines 1–50, then make the following change: [describe change]. Verify the build after."

**Architect then implement**
> "/architect Redesign [component] so that [goal]. Give me a plan with target files, ordered steps, verification, and risks before touching any code."
> *(After reviewing the plan, run:)* `/implement-plan`

**Code review — read only**
> "/ask Review `src/agent/inference.rs`. Identify any error paths that silently swallow errors, any retry logic that could loop indefinitely, and any place where context could be corrupted. Do not propose edits — just report."

**Git workflow**
> "Show me the diff of everything uncommitted, summarize what changed and why it matters, then write a conventional commit message for it."

**Branch audit**
> "Check the current branch state: uncommitted changes, commits ahead of main, and whether anything looks risky to merge. Use `repo_doctor`."

**LSP diagnostics**
> "/lsp — then read the active diagnostic errors and tell me which ones are real bugs versus noise, and which file is the highest priority to fix."

**Performance verification**
> "Implement [feature], then run a disk benchmark to verify it hasn't introduced significant disk queue spikes compared to the baseline."

**Fix plan**
> "I'm seeing [describe error or symptom]. Run a fix plan — inspect the relevant machine state first, then give me a numbered remediation checklist grounded in what you actually observe."

---

## Session and Memory

**Session handoff**
> "Summarize the key decisions and open questions from the last few sessions. What is the current working focus?"

**Retrieve past decisions**
> "Search session memory for any architectural decisions we made about [topic]. I want the reasoning, not just the outcome."

**Find past bugs**
> "Search session memory for any reported bugs or problems related to [component]. Did we resolve them or are they still open?"

**Imported knowledge retrieval**
> "Analyze the imported chat history in `.hematite/imports`. Are there any recurring patterns or unresolved bugs across sessions?"

**Workspace profile**
> "/workspace-profile"

**Vein memory inspection**
> "/vein-inspect"

**View active behavioral rules**
> "/rules view"

**Edit project-wide rules**
> "/rules edit shared — then add simplicity-first and surgical-edit guidelines"

**Edit personal local rules**
> "/rules edit — personal overrides, gitignored, apply on next turn"

**Repo doctor**
> "Run a repo doctor on this workspace. Check git status, uncommitted changes, branch state, remote tracking, and whether the build files look healthy."

---

## Think Mode

**Complex architectural trade-off**
> "/think Should [component A] own [responsibility] or should it live in [component B]? Walk me through the trade-offs — coupling, testability, context cost — before recommending."

**Multi-step debugging**
> "/think I'm seeing [symptom] only under [condition]. Think through what execution paths could produce this — don't jump to a fix until you've ruled out at least three causes."

**Design review**
> "/think Read `src/agent/conversation.rs` and `src/agent/inference.rs`. Is the current ownership boundary between them correct, or is there cohesion that belongs in one file bleeding into the other?"

**Risk assessment before a big change**
> "/think Before we refactor [component], think through what could go wrong: hidden callers, shared state, serialisation format changes, test coverage gaps. Give me a risk list."

---

## Document and Image Attachments

**Attach a spec or doc**
> *(Press `Ctrl+O` or type)* `/attach path/to/spec.pdf`
> "I've attached a spec. Read it and tell me which parts map to existing code in this repo and which parts are not yet implemented."

**Attach a PDF that's failing extraction**
> "The PDF I need to reference has custom fonts that break text extraction. I'll send it as an image instead."
> *(Then press `Ctrl+I` or type)* `/image path/to/screenshot.png`

**Screenshot a bug**
> *(Press `Ctrl+I` or type)* `/image path/to/screenshot.png`
> "This is a screenshot of the error I'm seeing. Read it and tell me what's wrong."

**Architecture diagram**
> *(Attach diagram image, then:)*
> "I've attached an architecture diagram. Map the components in it to actual files in this repo."

---

## Worktree

**Isolated feature branch**
> "/worktree feature/my-new-feature — set up an isolated worktree so I can work on this without touching main."

**Safe experiment**
> "/worktree experiment/try-this-refactor — I want to try a destructive refactor without risking the main working tree."

---

## Windows Admin Deep-Dives

**Storage overview**
> "Show me all drives with capacity and usage bars, flag any developer cache directories that are growing large, and give me the current real-time disk queue depth."

**BitLocker and encryption**
> "Check BitLocker status on all drives — protection state, encryption percentage, and any volumes that are unprotected."

**Certificates audit**
> "List my local personal certificates with subjects, thumbprints, and expiry dates. Flag anything expiring within 30 days."

**Shadow copies and restore points**
> "Show me my VSS snapshot history, storage allocation for shadow copies, and recent system restore points."

**Page file and virtual memory**
> "Show me the page file configuration, current usage, and peak usage. Tell me if the sizing looks healthy relative to my RAM."

**RDP and remote access**
> "Check whether RDP is enabled on this machine, what port it's on, whether NLA is required, and whether the firewall rule is active."

**Shares and SMB**
> "Show me every SMB share this machine is exposing, flag any non-admin custom shares, and tell me whether SMB1 is enabled."

**User accounts and sessions**
> "List all local user accounts, their enabled state, last logon, and which ones are in the Administrators group. Also show me any active logon sessions."

**Audit policy**
> "Show me which Windows audit policy categories are currently logging Success or Failure events. Flag if nothing is being audited."

**Scheduled tasks**
> "List all non-disabled scheduled tasks with their last run time and executable path. Flag anything that looks non-standard."

**Installed software**
> "Show me all installed programs on this machine with versions. Flag anything that looks outdated or unexpected."

---

## Teacher Mode

**Driver install walkthrough**
> "/teach How do I install a specific device driver on Windows without breaking anything? Inspect the machine state first."

**WSL setup**
> "/teach Walk me through setting up WSL2 and installing Ubuntu on this machine."

**SSH key generation**
> "/teach How do I generate an SSH key, add it to ssh-agent, and configure it for GitHub on this machine?"

**Firewall rule**
> "/teach Show me how to create an inbound Windows Firewall rule to allow traffic on port 8080."

**Service configuration**
> "/teach How do I configure a Windows service to restart automatically on failure? Inspect what's currently running first."

---

## SysAdmin & Network Engineering

**NTFS Permissions Audit**
> "Audit the permissions for the Downloads folder and tell me if any non-admin users have write access."

**Authentication & Login History**
> "Who logged into this machine over the last 48 hours? Flag any failed logon attempts if you find them."

**Network Share Accessibility**
> "Can I reach the network share `\\backup-server\archives` right now? Test connectivity and readability."

**Persistence & Security Audit**
> "Audit my registry for persistence hijacks (IFEO, Winlogon Shell, BootExecute) and verify if any debuggers are attached to system processes."

**Accurate Resource Telemetry**
> "Which processes are using the most CPU % right now? Show me real-time Task Manager style metrics, not cumulative seconds."

**Network Throughput (Rate-based)**
> "Show me my current network throughput in Mbps for the active adapter. I want to see the current RX/TX rate, not just totals since boot."

**Full network stack triage**
> "Run a complete network stack triage: latency to gateway and internet, DHCP lease status, MTU per adapter, DNS nameservers, and active TCP connections."
> *(Harness pre-runs: latency, dhcp, mtu, dns_servers, connections)*

**IPv6 deep-dive**
> "Give me a full IPv6 diagnostic: addresses per adapter, SLAAC vs DHCPv6, default gateway, privacy extension state, and any tunnel adapters."
> *(Harness runs: ipv6)*

**TCP stack health**
> "Audit TCP stack settings: autotuning level, congestion algorithm, ECN capability, chimney offload, and dynamic port range."
> *(Harness runs: tcp_params)*

**Wireless security audit**
> "Audit all saved wireless profiles for weak authentication (WEP, open). Show me what's currently connected and at what signal strength."
> *(Harness runs: wlan_profiles, wifi)*

**VPN and tunnel audit**
> "Check for active VPN adapters, IPSec security associations, and IKE tunnel state. Is anything tunneling traffic right now?"
> *(Harness runs: vpn, ipsec)*

**Time-Windowed Log Analysis**
> "Show me all System errors from the Event Log that occurred in the last 4 hours."
> "Show me Event ID 4625 failures from the Security log in the last 24 hours."
> "Search the System log for Event ID 7034 service crashes in the last 6 hours."

**Microsoft 365 Identity Audit**
> "Check Microsoft 365 sign-in health on this machine."
> "Audit token broker, Web Account Manager, and device registration."
> "Why won't Outlook sign in and why does Teams keep asking me to authenticate?"

**Diagnostic Analysis and Performance**
> "Why is my laptop slow? Check if it is overheating, throttling, under heavy I/O pressure, or if my silicon clocks are fluctuating."
> *(Harness automatically runs: thermal, resource_load, storage, overclocker)*

**Licensing & Patch Audit**
> "Is my Windows license valid? Check activation status and license details."
> "What changed in the last 48 hours? Show me the patch history and recently installed hotfixes (KBs)."

---

## IT Pro Plus Diagnostics

**Active Directory User Identity Investigation**
> "Analyze this domain user identity: show me their SID, enabled status, password expiration date, and all group memberships. Flag if the account is expired or disabled."
> *(Harness runs: ad_user)*

**Service Discovery via DNS SRV/MX**
> "Perform a high-precision DNS lookup for our domain controllers and mail servers. I want to see the SRV, MX, and TXT records to verify service discovery is working correctly."
> *(Harness runs: dns_lookup)*

**Hyper-V VM Inventory and Load**
> "Audit my local Hyper-V environment: list all virtual machines, their current execution state, uptime, and real-time CPU/Memory load stats."
> *(Harness runs: hyperv)*

**Precision Network Triage (IP/DHCP)**
> "Give me a deep-dive IP configuration report for all active adapters. I need to see DHCP server addresses, lease acquisition/expiry times, and any secondary IP aliases."
> *(Harness runs: ip_config)*

**Service Account Audit**
> "List all running Windows services and tell me which service accounts they are logging on as. Flag any services running as actual local users instead of System/NetworkService/LocalService."
> *(Harness refined: services)*

**Scheduled Task Failure Triage**
> "Audit all scheduled tasks. I want to see the Last Run Time and the numeric Result Code for each. Flag any task that has a non-zero exit result."
> *(Harness refined: scheduled_tasks)*

**High-Performance Silicon Audit**
> "Run a high-fidelity silicon performance audit. I need the 2-second average for CPU clock stability and deep NVIDIA metrics for voltage and power limits."
> *(Harness runs: overclocker)*

---

## Swarm

**Multi-file refactor**
> "/swarm Rename the `loop_intervention` field to `turn_injection` across all files in `src/agent/`. Each worker should handle one file."

**Parallel diagnostics**
> "/swarm Run a read-only audit of the top five architecturally important files in this repo. Each worker should summarize one file's role and flag anything that looks out of place."
