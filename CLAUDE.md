# Hematite CLI Documentation

## What this project is

Hematite is a local AI coding harness and natural-language workstation assistant built in Rust. It runs on your machine and uses any OpenAI-compatible local model server. The default target is LM Studio on `localhost:1234`, but the endpoint is configurable. The terminal TUI is one interface layer of the product, not the whole product. The main engineering target is a single-GPU consumer Windows setup, especially RTX 4070-class hardware.

Hematite supports two model protocol paths:

- **Gemma 4 native** — Gemma 4 family models; native tool markup auto-enabled by model name (`gemma_native_auto: true` by default)
- **Standard OpenAI-compatible** — all other models; plain tool format; tested primary target is Qwen/Qwen3.5-9B Q4_K_M

## Build and Run

```powershell
cargo build
cargo run
cargo run -- --no-splash
cargo run -- --rusty
cargo run -- --yolo
cargo run -- --brief
cargo run -- --stats
pwsh ./clean.ps1
```

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
- `@` in input: opens live file autocomplete — scans workspace, filters as you type, Tab/Enter inserts the path
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
- `/ask [prompt]`: sticky read-only analysis mode
- `/code [prompt]`: sticky implementation mode
- `/architect [prompt]`: sticky plan-first mode that persists a reusable handoff
- `/implement-plan`: execute the saved architect handoff in `/code`
- `/read-only [prompt]`: sticky hard no-mutation mode
- `/teach [prompt]`: sticky teacher mode — inspects real machine state first, then delivers a grounded numbered walkthrough for any admin/config/system task; does not execute write operations itself
- `/auto`: return to the narrowest-effective workflow mode
- `/vein-inspect`: inspect indexed Vein memory, hot files, and active room bias
- `/workspace-profile`: inspect the auto-generated workspace profile
- `/version`: show the running Hematite release version plus build state
- `/about`: show author, repo, and product info
- `hematite --version`: print the same build report from the CLI
- `/swarm`: trigger parallel worker agents

Requires LM Studio running locally with a model loaded and the server started on port `1234`.

Practical rule: the version/build label is compile-time metadata. A new commit or tag does not change what the already-built binary reports. Rebuild the binary or rerun `pwsh ./scripts/package-windows.ps1 -AddToPath` if you want `hematite --version`, `/version`, and the startup banner to reflect the latest commit, tag, or dirty/clean state.

Attribution rule: authorship and identity questions should resolve from Hematite's fixed app metadata, not from model improvisation. `/about` is the operator-facing path, and prompts like `who created you` or `who engineered Hematite` should answer with Ocean Bennett as the creator and maintainer.

Package naming rule: the crates.io package is `hematite-cli`, but the executable name stays `hematite`. Keep that split so the package namespace is distinct while the operator command stays short.

Crates.io publish order: publish `hematite-kokoros` first, then publish `hematite-cli`. The main package depends on the forked voice crate by published package name while keeping the source-level crate path as `kokoros`.

Crates.io compatibility rule: the default published/source build does not embed the large Kokoro voice assets. Packaged releases and local packaging scripts must build with `--features embedded-voice-assets` so the shipped Windows/macOS/Linux bundles keep the baked-in voice engine.

Crates.io update rule: in normal use, almost every public tagged Hematite release should republish `hematite-cli`. Republish `hematite-kokoros` only when the vendored fork itself changes. Do not bump the voice fork just because the main app shipped a new release.

- **Host Inspection Priority**: For all diagnostic questions (load, network, processes, log-checks), the agent **MUST** prefer `inspect_host` over raw `shell`.
- **Global Guard Enforcement**: Any raw shell diagnostic (wmic, Get-Process, tasklist) is blocked by a global linter in `conversation.rs` and redirected to `inspect_host`.
- **Telemetry Precision**: Use `topic: "resource_load"` for real-time CPU/RAM summaries and `topic: "processes"` for per-process cumulative CPU/Memory analytics.
- **Self-Correction Strategy**: When a diagnostic `shell` call is blocked, the agent must immediately self-correct to the designated `inspect_host` topic.
- **Harness Pre-Run**: When the user asks about 2+ inspection topics in one message, Hematite executes all queries *before* the model turn begins. Results are injected into the **conversation history** as simulated turns (assistant calls + tool results). This ensures the model (especially 9B variants) "sees" the data in the official transcript, which prevents redundant tool calls or orchestration loops.
- **Multi-Topic Rule**: Never collapse multiple distinct topics into a single generic topic like `"network"`. Each topic must be called separately. Example: "show route table, ARP, DNS cache, and traceroute" → four separate inspect_host calls (or one harness pre-run covering all four).
- **Storage Inspection**: Use `topic: "storage"` for all-drives capacity with ASCII bar charts and developer cache directory sizing (npm, cargo, pip, yarn, rustup, node_modules).
- **Hardware Inventory**: Use `topic: "hardware"` for full hardware DNA — CPU model/cores/clock, RAM total/speed/sticks, GPU name/driver/resolution, motherboard/BIOS, and display configuration.
- **Health Report**: Use `topic: "health_report"` (or alias `"system_health"`) for a tiered plain-English verdict (ALL GOOD / WORTH A LOOK / ACTION REQUIRED) across disk, RAM, tools, and recent error events.
- **Windows Update**: Use `topic: "updates"` for last install date, pending update count, and Windows Update service state.
- **Security Status**: Use `topic: "security"` for Defender real-time protection, last scan age, signature freshness, firewall profile states, Windows activation, and UAC state.
- **Pending Reboot**: Use `topic: "pending_reboot"` to check if a restart is queued (Windows Update, CBS, file rename operations) and why.
- **Drive SMART Health**: Use `topic: "disk_health"` for physical drive health via Get-PhysicalDisk and SMART failure prediction.
- **Battery**: Use `topic: "battery"` for charge level, status, estimated runtime, and wear level — reports no battery gracefully on desktops.
- **Crash History**: Use `topic: "recent_crashes"` for BSOD/unexpected shutdown events and application crash/hang events from the Windows event log.
- **Scheduled Tasks**: Use `topic: "scheduled_tasks"` for all non-disabled scheduled tasks with name, path, last run time, and executable.
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
- **Traceroute**: Use `topic: "traceroute"` to trace the network path to a host (default 8.8.8.8). Accepts optional `host` arg. Uses `tracert` on Windows, `traceroute`/`tracepath` on Linux/macOS.
- **DNS Cache**: Use `topic: "dns_cache"` to inspect locally cached DNS entries — hostname, record type, resolved address, and TTL.
- **ARP Table**: Use `topic: "arp"` for the ARP neighbor table — IP-to-MAC mappings for devices on the local network.
- **Route Table**: Use `topic: "route_table"` for the system routing table — destination prefixes, next hops, metrics, and interface names.
- **Environment Variables**: Use `topic: "env"` to inspect environment variables — shows developer/tool vars (CARGO_HOME, JAVA_HOME, GOPATH, etc.) and redacts secret-shaped values (KEY, TOKEN, PASSWORD) to presence-only.
- **Hosts File**: Use `topic: "hosts_file"` to read `/etc/hosts` (Windows: `drivers\etc\hosts`) — active entries, custom non-loopback entries flagged, full file content shown.
- **Docker**: Use `topic: "docker"` for Docker daemon state, running containers, local images, Compose projects, and active context. Reports gracefully if Docker is not installed or daemon is not running.
- **WSL**: Use `topic: "wsl"` for Windows Subsystem for Linux — installed distros, running state, WSL version. Windows-only; reports gracefully on Linux/macOS.
- **SSH**: Use `topic: "ssh"` for SSH client version, sshd service state, `~/.ssh` directory inventory (known_hosts count, authorized_keys count, private key files), and `~/.ssh/config` host entries.
- **Installed Software**: Use `topic: "installed_software"` for installed programs — winget list on Windows (registry fallback), dpkg/rpm/pacman on Linux, brew + mas on macOS.
- **Git Config**: Use `topic: "git_config"` for global git configuration audit — user identity, core settings, signing config, push/pull defaults, credential helper, branch defaults, local repo config, and git aliases.
- **Databases**: Use `topic: "databases"` to detect running local database engines — PostgreSQL, MySQL/MariaDB, MongoDB, Redis, SQLite, SQL Server, CouchDB, Cassandra, Elasticsearch — via CLI version check, TCP port probe, and OS service state. No credentials required.
- **User Accounts**: Use `topic: "user_accounts"` for local user accounts (name, enabled state, last logon, password required), Administrators group members, active logon sessions, and whether the current process is running elevated.
- **Audit Policy**: Use `topic: "audit_policy"` for Windows audit policy (auditpol /get /category:*) — shows which event categories are logging Success/Failure. Flags if no categories are enabled. Requires Administrator elevation on Windows; falls back to auditd on Linux.
- **Shares**: Use `topic: "shares"` for SMB shares this machine is exposing (flags custom non-admin shares), SMB server security settings (SMB1/SMB2 state, signing required, encryption), and mapped network drives. Warns if SMB1 is enabled.
- **DNS Servers**: Use `topic: "dns_servers"` for the DNS resolvers configured per network adapter (not cache — the actual configured nameservers), annotated with well-known providers (Google, Cloudflare, Quad9, OpenDNS), DoH configuration, and DNS search suffix list.
- **Teacher Mode (`/teach`)**: Activates a grounded walkthrough mode for write/admin tasks Hematite cannot safely execute itself. Protocol: (1) call `inspect_host` with the relevant topic(s) to observe actual machine state, (2) deliver a numbered step-by-step tutorial referencing real observed state — exact commands, exact paths, exact values. Does NOT execute write operations. Covers: driver install, Group Policy, firewall rules, SSH key generation, WSL setup, service config, Windows activation, registry edits, scheduled tasks, disk cleanup.
- **Fix Plan Teacher Lanes**: `topic: "fix_plan"` accepts an `issue` string and routes to the matching grounded walkthrough. New lanes: `driver_install`, `group_policy`, `firewall_rule`, `ssh_key`, `wsl_setup`, `service_config`, `windows_activation`, `registry_edit`, `scheduled_task_create`, `disk_cleanup`. Each lane inspects real machine state first, then delivers machine-specific numbered steps.

## Hardware Intent

Hematite is not trying to outscale cloud agents. It is trying to make a single local consumer GPU perform as well as possible for real coding work.

- Primary target: one RTX 4070-class GPU with roughly 12 GB VRAM
- Main engineering constraints: limited local context, open-model inconsistency, and VRAM pressure under long sessions
- Design response: stronger tooling, grounded traces, compaction, retrieval, and operator workflow instead of pretending the model is smarter than it is

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

**Workspace profile.** Hematite also writes `.hematite/workspace_profile.json` on startup. It is a
gitignored, auto-generated project profile containing detected stack/package-manager hints,
important folders, ignored noise folders, and build/test suggestions. The prompt can use it as
lightweight grounding before the model starts guessing about repo shape. Use `/workspace-profile`
to inspect the current generated profile in the TUI.

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

**Per-project database:** stored at `.hematite/vein.db` inside the workspace root. Each project
folder gets its own index. The Vein learns from files on disk and local session artifacts, not from
cloud state.

**Non-project directories:** when Hematite is launched outside a real project (no `Cargo.toml`,
`package.json`, `go.mod`, etc. found walking up from the launch directory), it skips the source-file
walk but still keeps The Vein active in docs-only mode. `.hematite/docs/`, imported chats in
`.hematite/imports/`, and recent local session reports remain searchable, and the status badge
shows `VN:DOC`. A bare `.git` alone does not count
as a project workspace.

**Auxiliary local memory inputs:** besides project source, The Vein also indexes:

- `.hematite/docs/` for permanent local reference material
- `.hematite/reports/` for recent local session reports, chunked by exchange pair (`user` +
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
- Tool output overflow: when a tool result exceeds 8 KB, `cap_output_for_tool` writes the full text to `.hematite/scratch/<tool>_<timestamp>.txt` and returns a truncation notice with the scratch path; the model recovers the full content with `read_file` without repeating the original tool call; large `read_file` results under compact-context mode follow the same scratch path
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
- Session reports are written to `.hematite/reports/session_YYYY-MM-DD_HH-MM-SS.json` on every exit and cancel
- Report includes: session start timestamp, duration, model, context length, total tokens, estimated cost, turn count, and full transcript
- `.hematite/reports/` is gitignored — reports are local runtime artifacts
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

Do not bump just to test whether a feature works. For Hematite, the local portable is the pre-release smoke test. Public version bumps happen after the live local test passes.

`release.ps1` is for cutting a release from a known-good state. It is not a substitute for first validating an unshipped fix in the local portable.

For behavioral changes, diagnostics are part of the change, not optional cleanup. Prefer adding or updating focused coverage in `tests/diagnostics.rs` as you land the work so the live portable test is not your only proof.

**Solo verification loop (Codex/operator path):**

```powershell
cargo fmt
cargo check --tests
cargo test --test diagnostics
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

Regular clean removes runtime artifacts: ghost backups, scratch files, session memories, sandbox output, reports, and logs. Deep also removes build outputs and the vein database. `-PruneDist` is opt-in and removes stale packaged artifacts under `dist/` while keeping only the current `Cargo.toml` version. Reset goes further and wipes session state files — use this to simulate a first-run experience without touching `settings.json` or `mcp_servers.json`.

For Hematite, disk growth is a normal maintenance concern. This is a heavy native Rust project with release packaging, ORT/DirectML sidecars, tests, and repeated debug/release builds. `target/` can climb into the tens of gigabytes quickly, and after enough iteration it is believable to hit 50-100 GB of local build output. Treat periodic deep cleanup as part of the normal workflow. When disk pressure matters, run `pwsh ./clean.ps1 -Deep`; if you also want to keep only the latest packaged release artifacts, use `pwsh ./clean.ps1 -Deep -PruneDist`. Remember that the next full rebuild will be slower because you deliberately wiped cached build state.

## Contributor Roadmap

Hematite is designed around the real constraint of a single consumer GPU running 9B-class open models. The goal is not to pretend the local model is smarter than it is. The goal is to make the harness so tight that a 9B model on a 4070 can do real work.

This roadmap reflects that design philosophy: things that are worth doing now because they work with the model's actual capability, and things to revisit when local models improve.

### Tier 1 — High value, local models ready today

- **Streaming shell output** — `shell` blocks until completion; streaming would make long builds and test runs interactive and let the model see live output as it arrives.
- **Turn checkpointing** — persist a recovery spine (last goal, last tool call, last verification result) so a mid-session crash or `/new` does not silently erase context the model needed.
- **Whisper voice input** — Hematite already has a TTS output pipeline; Whisper for STT input would close the voice loop and make the harness hands-free.
- **Per-workspace model profiles** — let `.hematite/settings.json` specify a preferred model, context ceiling, and embed model per project; useful when different repos need different size/speed tradeoffs.
- **Smarter compaction** — current summary chaining loses ordering fidelity; a structured ledger (goal + tool sequence + verification result per turn) would compress more faithfully and survive longer sessions.

### Tier 2 — Worth doing when local models handle it reliably

- **Workflow engine** — encode multi-step coding workflows (read → edit → verify → commit) as explicit typed state machines that the harness drives, not the model re-plans each turn.
- **Tool dependency graph** — before executing a plan, check whether its tool sequence is valid (no write before read, no verify before edit). Block impossible plans before they waste a turn.
- **Context budget ledger** — track token cost per tool call and per turn; surface a real budget breakdown so the operator can see why a session hit the ceiling, not just that it did.
- **Multi-model routing** — for tasks that need a faster or smaller model (search, classification, label generation), route specific tool calls to a lightweight model while keeping the main session on the primary coding model.

### Tier 3 — Revisit when local 9B models catch frontier capability

- **The Vein as an explicit knowledge base** — manual `remember this` and `forget this` operator commands with durable, typed knowledge entries that survive `/new` and workspace resets.
- **Hardware-aware autonomy** — let the harness self-limit swarm fanout, tool parallelism, and context depth based on live VRAM and context-pressure readings without requiring operator intervention.
- **Privacy audit layer** — before `shell` or `run_code` runs, scan for credential patterns (API keys, tokens, env vars) in arguments and offer a redact-and-confirm path.
- **Session continuity across restarts** — a first-class carry-forward mechanism that resumes not just the transcript but the typed checkpoint state (goal, working set, last verification result) from the previous session in the same workspace.
