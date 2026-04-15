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

**Firewall rules**
> "List all active inbound firewall rules that allow traffic. Flag anything that looks non-default."

**Traceroute**
> "Trace the network path to 8.8.8.8 and tell me where the latency spikes are."

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
> "Read-only mode. Trace exactly how a user message travels from TUI input to the model and back. Use `trace_runtime_flow`. Name real channels, functions, and file references — do not guess."

**Targeted implementation**
> "Read-only mode first. Inspect `src/agent/conversation.rs` around the tool dispatch loop, then propose the minimal change needed to [describe goal]."

**Proof before edit**
> "/code Read `src/ui/tui.rs` lines 1–50, then make the following change: [describe change]. Verify the build after."

**Performance verification**
> "Implement [feature], then run a disk benchmark to verify it hasn't introduced significant disk queue spikes compared to the baseline."

**Architect a change**
> "/architect Redesign [component] so that [goal]. Give me a plan with target files, ordered steps, verification, and risks before touching any code."

---

## Session and Memory

**Session handoff**
> "Summarize the key decisions and open questions from the last few sessions. What is the current working focus?"

**Imported knowledge retrieval**
> "Analyze the imported chat history in `.hematite/imports`. Are there any recurring patterns or unresolved bugs across sessions?"

**Workspace profile**
> "/workspace-profile"

**Vein memory inspection**
> "/vein-inspect"

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

## Swarm

**Multi-file refactor**
> "/swarm Rename the `loop_intervention` field to `turn_injection` across all files in `src/agent/`. Each worker should handle one file."

**Parallel diagnostics**
> "/swarm Run a read-only audit of the top five architecturally important files in this repo. Each worker should summarize one file's role and flag anything that looks out of place."
