# hematite

**Your RTX 4070 is a serious coding machine. Hematite makes it one.**

Local AI coding agent for LM Studio. Runs entirely on your hardware. No API key, no cloud, no per-token billing. Reads your repo, edits files, runs builds, fixes errors, and speaks responses out loud — all from a single binary that boots in seconds.

**What it actually does:**
- Reads any file, grepping for the right location before touching anything
- Edits with exact-match precision, CRLF-safe on Windows
- Runs `verify_build` after every change and feeds errors back to the model automatically
- Ghost-snapshots every edit so `Ctrl+Z` restores the previous state instantly
- Indexes your codebase with hybrid BM25 + semantic search so the model starts each turn with the right code already in view
- Speaks every response through a built-in 54-voice TTS engine — statically linked, zero install, works offline
- Clean conversational chat mode alongside the full agent mode — terminal-native, no Electron, no browser

`hematite` is not a chat wrapper bolted onto an agent. It is a complete local AI interface: coding agent when you need it, clean conversation when you don't. LM Studio handles model serving. Hematite handles everything else.

![Version](https://img.shields.io/badge/version-0.1.0-orange?style=flat-square)
![Windows](https://img.shields.io/badge/Windows-native-blue?style=flat-square)
![Linux](https://img.shields.io/badge/Linux-supported-green?style=flat-square)
![macOS](https://img.shields.io/badge/macOS-supported-lightgrey?style=flat-square)
![Offline](https://img.shields.io/badge/offline-100%25-brightgreen?style=flat-square)
![Voice](https://img.shields.io/badge/voice-54_voices-purple?style=flat-square)
![RAG](https://img.shields.io/badge/RAG-hybrid_BM25+semantic-blue?style=flat-square)
![LM Studio](https://img.shields.io/badge/LM_Studio-native-blueviolet?style=flat-square)
![License](https://img.shields.io/badge/License-AGPL--3.0-blue?style=flat-square)

---

## Contents

- [Why Hematite Wins Its Lane](#why-hematite-wins-its-lane)
- [One Binary. No Runtime. No Drama.](#one-binary-no-runtime-no-drama)
- [Terminal-Native Chat](#terminal-native-chat--better-than-a-browser-ui)
- [Hematite vs The Field](#hematite-vs-the-field)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [What It Can Do](#what-it-can-do)
- [Key Features](#key-features)
- [Voice](#voice)
- [TUI Slash Commands](#tui-slash-commands)
- [Configuration](#configuration)
- [MCP Servers](#mcp-servers)
- [Distribution](#distribution)
- [Cleanup](#cleanup)

---

## Why Hematite Wins Its Lane

Most local AI coding tools are either:

- cloud products with a local mode bolted on later
- generic local chat shells with weak repo grounding
- Linux-first tools that quietly fall apart on Windows

Hematite is built around the opposite assumption: the best local coding agent should feel native on the machine you actually use, tell the truth about its runtime state, and survive the constraints of real consumer GPUs instead of pretending they do not exist.

- **Proven tool loop** — read → grep → edit → verify → undo. Tested on real Rust codebases. The model finds the right line, makes the right change, catches its own errors, and recovers without prompting.
- **Sandboxed code execution** — the model can write and run JavaScript or Python in a zero-trust sandbox (no network, no filesystem escape, hard timeout). Real answers from real computation — not training-data guesses. Hematite automatically uses Deno from LM Studio's install — no extra setup required. Not using LM Studio? Install Deno globally: `winget install DenoLand.Deno`.
- **Hybrid RAG built in** — The Vein indexes your codebase with BM25 + semantic search (optional nomic-embed). The model starts each turn with the relevant code already loaded, not hunting for it with five read_file calls.
- **Built-in voice, zero install** — 54-voice Kokoro TTS, statically linked. No Python, no DLL hunting, no model downloads. Press `Ctrl+T`. Works on first run.
- **Windows-first** — CRLF-safe edits, PowerShell shell, Windows path handling, tested on RTX 4070. Not a Linux port with rough edges.
- **Honest about hardware** — VRAM usage in the status bar, context budget visible, compaction triggered before you hit the ceiling. No silent failures.
- **Zero ongoing cost** — no API key, no subscription, no per-token billing. Run it all day.
- **Private by default** — nothing leaves your machine. No telemetry, no cloud fallback.

**Windows is the primary development target.** PowerShell integration, path handling, shell behavior, and sandbox isolation receive the most polish there. Linux and macOS are supported.

---

## Why People Actually Use It

Hematite is for developers who want a **local coding CLI that behaves like a serious tool**, not a toy shell around a model server.

- You want a **local coding agent for LM Studio** that can read, edit, search, verify, and reason about a real repository.
- You want a **Windows local AI coding assistant** that does not treat PowerShell or local pathing as second-class.
- You want a **local coding harness** that survives tight context budgets instead of silently melting down near the ceiling.
- You want a **local coding CLI for RTX 4070-class hardware** that admits hardware limits and engineers around them.
- You want a harness that exposes **runtime truth**: live model context, budget pressure, recovery steps, blocker states, and session carry-forward.

If your goal is cloud-scale autonomous orchestration, Hematite is not trying to win that game. If your goal is the best practical local harness for repo work on consumer hardware, that is exactly the category it is trying to own.

---

## One Binary. No Runtime. No Drama.

Most local AI tools make you pay an installation tax before you get anything useful:

- **AnythingLLM / Jan / Open WebUI** — Electron apps. 200–400 MB installers, a browser engine running in the background, Node.js or Python under the hood. They look like apps because they are apps — with all the RAM overhead and startup lag that comes with it.
- **Python-based tools** — require a matching Python version, a virtual environment, pip dependencies that break across OS updates, and often a separate CUDA toolkit install.
- **Docker-wrapped tools** — need Docker Desktop running, eat RAM on idle, and add a layer of indirection between you and your GPU.

Hematite is a single native binary written in Rust.

**What that means for you, even if you've never heard of Rust:**

- **No runtime to install.** Drop the `.exe` in a folder and run it. That's the whole install.
- **Boots in under a second.** No VM warmup, no Electron splash screen, no Node module loading.
- **Uses ~30 MB of RAM at idle** instead of 300–600 MB for an Electron app doing the same job.
- **Won't break on Windows updates.** There's no Python interpreter to conflict, no Node version to mismatch, no DLL hell. The binary carries everything it needs except `DirectML.dll`, which Windows ships by default.
- **The voice engine is inside the binary.** 311 MB ONNX model, 54 voices, ONNX Runtime 1.24.2 — all compiled in at build time. No model download on first run, no version mismatch, no separate TTS service.
- **Smaller than most app installers.** The entire Hematite portable — binary + voice model + DirectML — is ~336 MB as a zip. AnythingLLM's installer alone is larger than that.

Rust produces machine code that runs directly on your CPU, the same as C or C++, with no interpreter in between. You get the startup speed of a native app, the memory footprint of a CLI tool, and the reliability of software that can't silently crash a garbage collector mid-turn.

---

## Terminal-Native Chat — Better Than a Browser UI

Hematite runs in your terminal. That's not a limitation — it's a feature.

Tools like AnythingLLM and Jan give you a browser UI because that's the fastest way to ship something that looks polished. But a browser UI means:

- Electron eating 200–500 MB RAM before the model even loads
- No keyboard-first workflow
- Copy-paste friction between your editor and the AI
- A separate window you're constantly Alt-Tabbing to

Hematite lives where your code lives — in the terminal, inside your project folder. You open it, talk to it, and close it. No context switching, no separate app window, no mouse required.

**Agent mode** — full coding harness: reads files, edits with precision, runs builds, recovers from errors, indexes your repo with RAG.

**Chat mode** — clean conversational surface: no tool noise, no agent scaffolding, just a fast response loop with voice. The same Vein RAG runs underneath so the model still knows your codebase.

Both modes. One binary. Voice in both. Switch with a command.

---

## Hematite vs The Field

There are several tools in this space. Here is what each one actually requires and what it actually does.

| | **Hematite** | **Aider** | **Continue** | **AnythingLLM / Jan** | **Open Interpreter** |
|---|---|---|---|---|---|
| **Install** | Single `.exe`, no runtime | `pip install aider` + Python 3.10+ | VS Code/JetBrains extension | Electron installer (200–400 MB) | `pip install open-interpreter` + Python |
| **Local models** | First-class (LM Studio, Ollama) | Supported but secondary | Supported | Supported | Supported but secondary |
| **Cloud-first** | No | Yes (GPT-4/Claude default) | No | No | Yes (GPT-4 default) |
| **Cost** | Free, offline | Free tool, pay per token if cloud | Free | Free | Free tool, pay per token if cloud |
| **Windows quality** | Native, tested, CRLF-safe | Workable, Linux-first design | IDE-dependent | Electron (cross-platform) | Workable |
| **Codebase RAG** | Built-in (BM25 + semantic) | No | Basic | Basic | No |
| **Voice / TTS** | Built-in, 54 voices, offline | No | No | No | No |
| **Chat mode** | Yes (agent + clean chat) | No (agent only) | Yes (IDE chat) | Yes (browser UI) | No |
| **Idle RAM** | ~30 MB | ~50 MB | IDE overhead | 200–500 MB (Electron) | ~80 MB |
| **Build verification** | Built-in, error recovery loop | Git-diff focused | No | No | Ad hoc shell |
| **Code execution** | Sandboxed JS + Python (zero-trust) | No | No | No | Yes (primary feature) |
| **Undo / ghost backup** | Built-in (`Ctrl+Z`) | Git-based | No | No | No |
| **Offline** | Fully offline | Fully offline | Fully offline | Fully offline | Fully offline |

### The honest breakdown

**Aider** is the closest real competitor in the terminal space. It's well-engineered and git-native. It does not have RAG, voice, a TUI, Windows-native polish, or built-in build verification. It assumes cloud models by default. If you're on Linux and already have Python, it's a solid choice for git-centric workflows. If you're on Windows, running local models, and want voice + codebase awareness, Hematite is the better fit.

**Continue** lives inside your IDE. That's its strength (tight editor integration) and its ceiling (you're inside VS Code's Electron process, not a standalone agent). It has no build verification loop, no RAG of its own, and no voice.

**AnythingLLM and Jan** are UI-first products. They look polished because they're browser apps. You pay for that with 200–500 MB RAM at idle, Electron startup lag, and no real terminal workflow. They're good for people who want a chat interface and aren't doing serious coding work with the AI.

**Open Interpreter** is the closest thing to Hematite's ambition — a local agent that can run code and talk to your system. It requires Python and defaults to cloud. It has no codebase RAG, no voice, and no Windows-specific polish. Hematite now matches its code execution capability with a sandboxed `run_code` tool (JS + Python, zero-trust, hard timeout) while adding everything Open Interpreter lacks.

**The real gap:** none of these tools have all four of Hematite's core differentiators at once — offline-capable codebase RAG, built-in voice, sandboxed code execution, and a native binary with no runtime dependency. That combination doesn't exist anywhere else in this category right now.

---

## Product Boundary

Hematite is the **agent harness**.
LM Studio is the **local inference runtime**.

Hematite handles:
- terminal UI and operator workflow
- tool calling, editing, shell, and git execution
- local retrieval, compaction, and context shaping
- voice, GPU awareness, and multi-step orchestration

LM Studio handles:
- loading local models
- swapping models quickly
- updating runtimes and models without rebuilding Hematite
- serving the OpenAI-compatible endpoint on your machine

That split is intentional. Hematite focuses on being the best local coding harness; LM Studio focuses on model lifecycle.

---

## Autonomy Model

Hematite is designed as a high-agency coding partner with bounded autonomous lanes, not as a claim that a 10 GB local model should behave like a frontier cloud worker on every task.

That product direction is deliberate:

- local open models are useful, but they are less reliable under ambiguity, recovery, and long tool sequences
- single-GPU setups need tighter context discipline, fewer wasted calls, and more deterministic recovery
- the harness can own repetitive workflow logic more cheaply and more reliably than the model can infer it every turn

So Hematite leans into micro-workflows:

- inspect before edit
- confirm the local window before mutating
- verify after code changes
- clamp recovery when the model starts guessing
- preserve operator visibility instead of hiding failure

The model still matters. It should provide intent interpretation, code judgment, wording, and local reasoning between steps. But the harness should own the deterministic parts: tool sequencing, recovery ladders, context shaping, verification, and workflow guardrails.

That same pattern can be translated across the tool surface over time. The goal is not "less agent." The goal is better bounded autonomy in the places where local models are actually dependable.

---

## Requirements

| Platform | Shell | GPU Monitoring |
|---|---|---|
| Windows 10/11 | PowerShell (`pwsh` / `powershell.exe`) | NVIDIA via `nvidia-smi` |
| Linux | bash | NVIDIA via `nvidia-smi` |
| macOS | bash / zsh | Degrades gracefully |

- [LM Studio](https://lmstudio.ai) with a model loaded and the local server running on port `1234`
- NVIDIA GPU with 8 GB+ VRAM recommended; 12 GB VRAM is the sweet spot Hematite is most actively shaped around
- Rust toolchain if building from source

**Primary hardware target:** a single RTX 4070-class GPU on a normal desktop Windows machine. Hematite is engineered around that constraint: limited local VRAM, one active consumer GPU, LM Studio as the serving layer, and open models that need strong tooling and context discipline instead of cloud-scale brute force.

### Tested Model Configuration

This is the setup Hematite is actively developed and tested against:

| Role | Model | Size | VRAM |
|---|---|---|---|
| Coding agent | `Qwen/Qwen3.5-9B` (Q4_K_M quant) | ~5–6 GB | primary |
| Semantic search | `nomic-embed-text-v2` Q8_0 | ~512 MB | secondary |

Load both in LM Studio at the same time. The embedding model stays resident but is idle between turns — it doesn't compete with the coding model during inference.

**Why these two?** Qwen/Qwen3.5-9B Q4_K_M completed a full coding workflow (read → inspect → edit → verify → commit) in under 6000 tokens on RTX 4070 hardware. The nomic embedding model is small enough that both fit in 12 GB VRAM with room to spare, and it enables semantic retrieval in The Vein so Hematite can find the right file before the model even asks.

**Gemma 4 is also supported** with a native protocol path (auto-detected by model name). The standard OpenAI-compatible path (Qwen and others) is the primary tested configuration.

---

## Quick Start

### Fastest Summary

1. Install [LM Studio](https://lmstudio.ai).
2. Load `Qwen/Qwen3.5-9B` Q4_K_M (or any compatible model) and start the local server on port `1234`.
3. Optionally load `nomic-embed-text-v2` alongside it for semantic file search.
4. Launch `hematite` inside your project folder.

### Recommended User Path

1. Install LM Studio.
2. In LM Studio, download and load your coding model (tested: `Qwen/Qwen3.5-9B` Q4_K_M).
3. Also load `nomic-embed-text-v2` — this enables The Vein's semantic search and costs only ~512 MB VRAM. On a 12 GB card both models fit together.
4. Start the LM Studio local server on port `1234`.
5. Download a Hematite release bundle, or build from source with `cargo build`.
6. Launch `hematite` from inside your project folder.

### Developer Mode

```powershell
# 1. Build the engine
cargo build --release

# 2. Run from the project root
cargo run --release

# 3. Skip the splash screen for automation/tests
cargo run --release -- --no-splash

# 4. Skip approval modals (Approvals Off mode)
cargo run --release -- --yolo

# 5. Show your Rusty companion stats and exit
cargo run --release -- --stats
```

---

## Distribution

Hematite is designed as a **workspace-aware standalone executable** that pairs with LM Studio.

### Building a Release

```powershell
pwsh ./release.ps1
```

This builds `--release`, copies `hematite.exe` and `DirectML.dll` into `dist/windows/Hematite-0.1.0-portable/`, and rezips the portable archive. Output is ~336 MB (voice model is baked in).

To build with a different version tag:

```powershell
pwsh ./release.ps1 -Version 0.2.0
```

### What ships in the portable

| File | Size | Purpose |
|---|---|---|
| `hematite.exe` | ~320 MB | Binary with ONNX Runtime + Kokoro voice model baked in |
| `DirectML.dll` | ~16 MB | GPU inference on Windows (auto-copied from build output) |
| `README.txt` | — | Quick-start instructions |

`dist/` is gitignored — these are release artifacts, not tracked in source.

### Automated Windows Releases

GitHub Actions can build the latest Windows installer and portable zip for you.

- `workflow_dispatch` lets you run the release build manually from GitHub
- pushing a tag like `v0.1.0` builds the newest Windows artifacts automatically
- tagged builds also attach the generated `.zip` and `Setup.exe` to the GitHub release

Typical release flow:

```powershell
git commit -am "release: prepare v0.1.1"
git tag v0.1.1
git push origin main --tags
```

Versioning still comes from `Cargo.toml`, so the package names and installer version stay aligned with the Rust crate version.

### Updating Hematite

Updating is as simple as replacing `hematite.exe` or installing a newer packaged release. Project-specific histories, rules, and task files live in each project's `.hematite/` directory and survive upgrades.

---

## What It Can Do

Hematite gives the loaded model a real local tool suite for coding work. This is the core difference between Hematite and a plain local chat shell:

| Tool | Description |
|---|---|
| `read_file` | Read any file with offset/limit pagination for large files |
| `write_file` | Write or overwrite files |
| `edit_file` | Find-and-replace edits with fuzzy whitespace matching |
| `multi_search_replace` | Precision engine for bulk find-and-replace blocks |
| `grep_files` | Regex search with context lines, files-only mode, and pagination |
| `list_files` | Directory listing with extension filtering |
| `map_project` | Compact architecture map with config markers, likely entrypoints, core owner files, and a bounded directory tree |
| `shell` | Run PowerShell commands with timeout and output capping |
| `research_web` | Run zero-cost technical web searches for docs, API changes, and debugging leads |
| `fetch_docs` | Fetch and convert documentation pages into readable Markdown for follow-up analysis |
| `vision_analyze` | Inspect screenshots, diagrams, and UI images with the multimodal model path |
| `trace_runtime_flow` | Return a grounded read-only trace of runtime control flow for architecture questions |
| `describe_toolchain` | Return a grounded read-only description of Hematite's real built-in tools and the right investigation order |
| `git_commit` | Stage all and commit with Conventional Commits style |
| `git_push` | Push to origin HEAD |
| `git_worktree` | Create, list, prune, and remove isolated worktrees |
| `verify_build` | Run build/test/lint/fix validation through verify profiles or auto-detected defaults |
| `clarify` | Ask the user a question when genuinely blocked |

---

## Key Features

### Dual Model Protocol Support

Hematite supports two model protocol paths. **Gemma 4 native**: auto-detected by model name; uses native control tokens such as `<|think|>`, `<|turn>`, and `<|tool_call>`, with tool results wrapped in `<|tool_response>` markup. **Standard OpenAI-compatible**: plain message format with no model-specific markup; tested primary target is Qwen 3.5 9B Q4_K_M. Internal reasoning is kept in a dedicated TUI panel instead of polluting the main chat.

### Hardware-Aware Context Management

Hematite reads GPU VRAM every 2 seconds. When memory pressure rises, it compacts earlier and caps parallel workers. The loaded model's context window is detected from LM Studio and injected into the system prompt so the model knows its own budget.

On startup, Hematite now prefers LM Studio's live `loaded_context_length` for the currently loaded model instead of relying on older `context_length` fields or a generic Gemma fallback. That matters when LM Studio serves a smaller active `n_ctx` than the model family could support on paper.

Hematite also refreshes the LM Studio runtime profile before each user turn. If you swap models or change the active loaded context in LM Studio mid-session, Hematite can pick up the new model ID and context budget without requiring a full harness restart.

Hematite now also runs a quiet background runtime-profile poll. The status bar can track live model and CTX changes even while you are idle, but chat/system messages are only emitted when the profile actually changes so the operator stays informed without constant noise.

The TUI status bar also exposes a compact LM runtime badge so you can tell at a glance whether Hematite sees LM Studio as live, stale, or under context-pressure without opening extra menus or reading chat spam.

Provider retries and hard runtime failures also flow through a compact provider-state path. Hematite can signal recovery, degraded runtime, or context-ceiling conditions through the badge and SPECULAR notes without turning every provider wobble into a long chat interruption.

Runtime-profile refreshes update the live model and CTX display, but they do not erase a real provider failure state by themselves. A badge like `LM:CEIL` or `LM:WARN` should persist until a genuinely successful turn clears it.

The status bar now also exposes a compact compaction-pressure badge (`CMP:NN%`). It is driven by Hematite's real adaptive compaction threshold, so you can see how close the conversation history is to the point where Hematite will start summary-chaining older turns.

Next to that, Hematite now exposes a prompt-budget badge (`BUD:NN%`). This is separate from compaction pressure: it reflects the full estimated turn payload against the live LM Studio context window, including the current prompt plus reserved output budget. That makes small-context failures easier to predict, especially when `CMP` is still low but the total prompt is already close to the ceiling.

The operator surface was also tightened up for local use: the professional status line is shorter, `ERR` is now a real session error counter instead of dead filler, the `TOKENS` badge uses the same warm palette as the rest of the harness, and the input footer now prioritizes controls that actually matter in the terminal (`Enter`, `Esc`, voice, approvals, `/help`) instead of wasting space on noisy or unreliable hints.

Provider-health state is now runtime-owned rather than inferred inside the TUI. The agent layer emits explicit provider-state events like `LIVE`, `RECV`, `WARN`, and `CEIL`, and the TUI only renders them. That keeps runtime refreshes, degraded retries, and recovery clears from fighting each other in the operator surface.

The same operator path now carries typed checkpoint/blocker states into SPECULAR, such as provider recovery, prompt-budget reduction, history compaction, blocked policy paths, blocked recent-file-evidence edits, and blocked exact-line-window edits. That means the runtime can surface what kind of recovery or blocker it hit without depending on whatever freeform thought text happened to be logged in that branch.

Hematite now also carries a typed recovery-recipe layer inspired by Claw's runtime model, but adapted for local LM Studio workflows. Instead of burying retries, budget reduction, compaction, runtime refresh, and proof-before-edit recovery behind scattered branches, the runtime maps those situations to named recovery scenarios and compact recovery steps like `retry_once`, `refresh_runtime_profile`, `reduce_prompt_budget`, `compact_history`, or `inspect_exact_line_window`. SPECULAR can surface those as `RECOVERY:` lines, and the same recipe state is written into the session ledger for later carry-forward.

Startup/runtime assembly is also cleaner now. Hematite no longer hand-builds its engine, channels, voice path, watcher, swarm coordinator, and runtime-profile sync directly inside `main.rs`. Those pieces are assembled through a typed runtime bundle in `src/runtime.rs`, while `main.rs` only boots the bundle, starts the agent loop, and launches the TUI. That keeps local-runtime ownership clearer without changing the operator-facing flow.

If LM Studio is serving a very small active context window such as 4k, Hematite now falls back to a tiny-context system prompt profile. That trims heavy scaffolding, skips bulky instruction and MCP sections, and keeps simple prompts like `who are you?` from failing before the model even gets a chance to answer.

If you want to force that sync manually, Hematite now exposes `/runtime-refresh` in the TUI. Context-window failures also trigger an immediate runtime-profile refresh so the operator can see whether LM Studio is still serving the same model and active context budget.

This is intentionally tuned around single-GPU consumer hardware. The design goal is not cloud parity; it is to get the best practical coding workflow out of a 4070-class local box.

### Adaptive Thought Efficiency

Using `--brief` or `/no_think`, Hematite injects low-effort reasoning instructions so simple tasks stay fast while deeper tasks can still use full thought depth.

### Recursive Compaction

When conversation history gets large, Hematite summary-chains older context instead of bluntly truncating it. The compaction threshold scales with context length and current VRAM usage.

Those recursive summaries are now budgeted and normalized before they go back into the prompt. Duplicate lines are removed, low-value lines are dropped first, and core lines like scope, key files, tools, and recent user requests are prioritized so summary chaining costs less on small local contexts.

### Ghost Commit System

Before every file edit, Hematite snapshots a hidden git ref at `refs/hematite/ghost`. `Ctrl+Z` in the TUI rolls back the last edit without touching branch history or the visible git log.

### Swarm Agents

`/swarm <directive>` spawns parallel worker agents that research, implement, and propose diffs for review. Worker count is capped automatically based on available VRAM.

**Single-GPU note:** on a single consumer GPU with one model loaded, workers share the same inference endpoint and run sequentially rather than truly in parallel. Swarm is most effective on multi-GPU setups or when pointing at a remote LM Studio instance. On a 4070-class machine it still works — it just runs workers one after another rather than simultaneously.

### Project Rule Discovery

Drop a `CLAUDE.md` or `.hematite.md` in your project root. Hematite picks it up automatically and follows your project-specific coding standards every turn.

### The Vein (Hybrid RAG Memory)

The Vein is Hematite's retrieval layer. At the start of every turn it re-indexes any changed files and queries for chunks relevant to the user's message. Results are injected directly into the system prompt so the model starts the turn with the right code already visible — reducing wasted `read_file` calls and letting smaller models perform better on unfamiliar codebases.

**The index is per-project.** The database lives at `.hematite/vein.db` inside the workspace root. Run Hematite in a different folder and it gets a completely separate index for that project — no cross-contamination. The index is purely file-driven: it learns from what's on disk, not from the conversation. As you edit code the index stays current automatically because files are only re-indexed when their mtime changes.

**Two retrieval modes run together:**

- **BM25 keyword search** (always on) — SQLite FTS5 with Porter stemming. Zero extra GPU cost, works with any LM Studio setup.
- **Semantic vector search** (optional, better) — Calls LM Studio's `/v1/embeddings` endpoint using `nomic-embed-text-v2`. Understands concept-level queries: "what renders the startup screen" finds the right function even if no file uses the word "banner". Vectors are stored in SQLite and reused across sessions so files are only re-embedded when they actually change.

**Why run two models?** The coding model handles language, reasoning, and tool calls. The embedding model does one specific job: convert code chunks and your queries into vectors so Hematite can find the most relevant files by meaning rather than just keywords. It stays loaded but idle during inference — it only activates when indexing changed files or searching. On an RTX 4070 (12 GB VRAM), Qwen/Qwen3.5-9B Q4_K_M uses ~6 GB and nomic-embed-text-v2 Q8_0 uses ~512 MB, leaving comfortable headroom. You do not need to swap models or manage them manually — just load both in LM Studio and leave them running.

**To enable semantic search:** load `nomic-embed-text-v2` Q8_0 in LM Studio alongside your main coding model. The status bar shows `VN:SEM` (green) when semantic search is active, `VN:FTS` (yellow) when only BM25 is running, and `VN:--` (grey) on a fresh session before the first index pass.

**Automatic backfill:** if you load the embedding model after Hematite has already indexed your project, it detects the gap and re-embeds the missing chunks gradually across the next few turns — no `/vein-reset` or file-touch needed. The `VN:FTS` badge flips to `VN:SEM` once the backfill completes.

Hybrid results are merged and ranked: semantic hits score higher when the embedding model is available; BM25 fills the gap when it isn't. Results are deduplicated by file path and capped so they don't crowd out the model's working context.

**To wipe and rebuild the index** (e.g. switching projects or after a big refactor): use `/vein-reset` in the TUI. The database is cleared immediately and rebuilt from scratch on the next turn. For a full deep clean including the database file, use `pwsh ./clean.ps1 -Deep`.

**Large file support:** the index covers files up to 512 KB — large source files like `tui.rs`, `inference.rs`, and `conversation.rs` are indexed in full, not silently skipped.

**BM25 query accuracy:** stopwords are filtered and tokens are OR-joined so conversational queries like "how does the specular panel work" return relevant results instead of empty matches.

**Backfill ordering:** when the embedding model is loaded after initial indexing, `.rs` source files are embedded before documentation and config files so the most useful chunks get semantic vectors first.

### Built-In Web Research

Hematite can search the web for technical information when local context is not enough. `research_web` is used to find likely documentation or debugging leads, and `fetch_docs` pulls the resulting pages into clean Markdown so the model can actually read them instead of guessing from snippets.

### Grounded Runtime Tracing

For architecture and control-flow questions, Hematite can use `trace_runtime_flow` to return a verified read-only runtime trace instead of relying on model memory alone. This is especially useful on local open models where exact symbol tracing is weaker than cloud frontier models.

Supported topics: `user_turn`, `session_reset`, `reasoning_split`, `runtime_subsystems`, `startup`, `voice`. The `voice` topic covers the Ctrl+T toggle binding, `VoiceManager.toggle()`, and the speech pipeline — returning a grounded answer in a single tool call instead of burning multiple turns on grep searches that return empty.

For broad read-only architecture walkthroughs, Hematite now pairs `trace_runtime_flow` with `map_project` instead of letting the model freestyle a long repo tour. The intended shape is: `map_project` for structure and owner files, one `trace_runtime_flow` topic for runtime or control flow, then a compact grounded overview.

### Grounded Tool Selection

For tooling-discipline and investigation-plan questions, Hematite can use `describe_toolchain` to return the real built-in tool surface, what each tool is good or bad for, and a concrete read-only investigation order. This helps local open models avoid inventing fake helper tools or fake symbol names when explaining how they would inspect a Rust codebase.

### Architecture-Aware Repo Mapping

`map_project` is not just a file tree. It returns a compact repo map with configuration markers, likely entrypoints, core owner files, and extracted top symbols so smaller local models can get useful spatial awareness before spending turns on deeper file reads or LSP queries.

### Architect -> Code Handoff

`/architect` can persist a compact implementation handoff into `.hematite/PLAN.md` and session memory. That handoff carries the goal, target files, ordered steps, verification action, risks, and open questions so `/code` can resume from a clean brief instead of reconstructing the plan from chat residue.

### Safe Gemma 4 Native Layer

Hematite now has a narrow Gemma 4 compatibility layer in the inference path. It does not rewrite the full conversation protocol. Instead, it detects Gemma 4 models and normalizes the specific malformed tool-argument patterns that local runs were actually producing, such as over-quoted `path` and `extension` fields or slash-delimited `grep_files` patterns.

That normalization also covers float-shaped numeric arguments produced by Gemma-style tool calls, so bounded reads like `limit: 50.0` or `context: 5.0` still stay bounded instead of silently expanding into full-file reads.

Provider-side preflight also now blocks oversized requests before they are sent to LM Studio, surfacing a `context_window_blocked` style error instead of silently hanging near the context ceiling.

When a local-model turn still degrades, Hematite now classifies the runtime failure into operator-facing buckets such as `context_window`, `provider_degraded`, `tool_policy_blocked`, `tool_loop`, and `empty_model_response` instead of surfacing random raw provider prose. Degraded LM Studio turns also get one automatic recovery retry before Hematite escalates the structured failure back to the operator.

That same structured failure treatment now covers the plain streaming path too, so startup and non-tool text generations do not silently die or leak raw provider errors when LM Studio degrades.

If LM Studio rejects a turn with a live context-budget mismatch such as `n_keep >= n_ctx`, Hematite now classifies that as a `context_window` failure instead of a generic provider degradation. The operator guidance also tells you to narrow the turn or re-detect the live model budget if LM Studio is serving a smaller context window than Hematite expected.

Tool authorization is now routed through a typed enforcement layer instead of a loose mix of config checks and ad hoc approval heuristics. That means shell rule overrides, MCP approval defaults, safe-path write bypasses, read-only denials, and shell risk classification all converge into one explicit runtime decision: allow, ask, or deny, each with a concrete source and reason.

Workspace trust is now part of that same runtime policy. Hematite resolves the current repo root as trusted, unknown, or denied instead of assuming every workspace deserves the same destructive-tool posture. A trusted workspace continues through normal policy checks, an unknown workspace can require approval for destructive or external actions, and a denied workspace can block them outright.

The tool registry also now carries more of its own runtime truth. Repo reads, repo writes, verification tools, git tools, architecture tools, workflow helpers, research tools, vision tools, and external MCP tools are no longer treated as one flat list in the orchestration layer. That metadata now informs plan scoping, parallel-safe execution, trust sensitivity, and mutability instead of relying only on scattered name checks.

That runtime tool surface is now owned more cleanly too: [src/agent/tool_registry.rs](/c:/Users/ocean/AntigravityProjects/Hematite-CLI/src/agent/tool_registry.rs) defines the built-in tool catalog and builtin-tool dispatch path, so [conversation.rs](/c:/Users/ocean/AntigravityProjects/Hematite-CLI/src/agent/conversation.rs) is less responsible for acting like a partial tool registry.

MCP health is also treated as first-class runtime state now. Hematite distinguishes between unconfigured, healthy, degraded, and failed MCP conditions so external server availability is visible to the operator without being confused with LM Studio model health.

Stable operator-help and product-truth questions now route through a small intent classifier instead of a long stack of unrelated prompt gates. In practice that means Hematite is less dependent on one exact wording when deciding whether a question is asking for stable product truth, runtime diagnosis, toolchain guidance, or repository architecture.

Session carry-over is also more explicit now. Hematite no longer preserves only the active task and working-set hints; it also carries typed session ledger entries for the latest checkpoint, latest blocker, latest recovery step, latest verification result, and latest compaction pass. That gives local-model sessions a better recovery spine after compaction instead of relying only on loose transcript residue.

For deeper experimentation, `.hematite/settings.json` supports two Gemma-native controls:

- `gemma_native_auto`: defaults to `true` and automatically enables the native-formatting path when the loaded model is Gemma 4
- `gemma_native_formatting`: defaults to `false` and force-enables the native-formatting path for Gemma 4 when you want to override the automatic mode explicitly

That native path is still limited to Gemma 4 models. When active, Hematite folds the system instructions into the first user turn instead of relying only on the legacy wrapper format. This is the safe way to test more native Gemma request shaping without changing the default runtime contract for every model.

If Hematite detects a Gemma 4 model at startup, it reports whether Gemma Native Formatting is `ON (auto)`, `ON (forced)`, or `OFF`. You can also control it live from the TUI with `/gemma-native auto`, `/gemma-native on`, `/gemma-native off`, or `/gemma-native status`.

### Vision Analysis

Hematite can inspect screenshots, diagrams, and UI captures through `vision_analyze`, which lets the model reason about visual bugs and interface state instead of relying only on text descriptions.

### Startup Greeting

On launch, Hematite prints a one-line status block:

```
Hematite Online | Model: qwen/qwen3.5-9b | CTX: 32000 | GPU: NVIDIA GeForce RTX 4070 | VRAM: 9.3 GB / 12.0 GB
Endpoint: http://localhost:1234/v1
Embed: nomic active (semantic search ready)
```

The endpoint line shows exactly where Hematite is connecting — immediately visible if you're using Ollama, a remote machine, or any non-default server instead of LM Studio.

### Background Audio Engine

Press `Ctrl+T` to enable real-time text-to-speech. Hematite ships a **self-contained voice engine** — no install, no downloads, no Python. The Kokoro model (311 MB), all 54 voices, and ONNX Runtime 1.24.2 are statically linked into the binary at compile time. On first start the voice engine loads in the background (~10–30s on an RTX 4070); you'll see "Vibrant & Ready" in the chat when it's done. Responses spoken during that window are buffered and played back once the engine is ready.

Voice settings are configurable via `/voice` or `settings.json`:

- `/voice` — list all 54 voices
- `/voice N` or `/voice <id>` — select a voice, saved immediately
- `voice_speed` (0.5–2.0×) and `voice_volume` (0.0–3.0×) in `.hematite/settings.json`

### Session Reports

On every exit (Ctrl+C) or cancel (ESC), Hematite writes a structured JSON report to `.hematite/reports/`:

```json
{
  "session_start": "2026-04-07_16-26-01",
  "duration_secs": 90,
  "model": "qwen/qwen3.5-9b",
  "context_length": 32000,
  "total_tokens": 5451,
  "estimated_cost_usd": 0.003,
  "turn_count": 6,
  "transcript": [...]
}
```

Reports are gitignored — they are local runtime artifacts for your own review.

### Tool Loop Guard

If the model calls the same tool with identical arguments 3 or more times in a single turn, Hematite injects a hard stop and tells the model to change approach. This prevents runaway grep/shell spirals that burn context without making progress. `verify_build` and git tools are exempt since repeated verification calls are legitimate in fix-verify loops.

### Windows Edit Reliability

`edit_file` and `multi_search_replace` normalize CRLF line endings before matching, so model-generated search strings (always LF) work correctly against Windows files. Prior to this fix, edits on CRLF files would fail the exact-match check and require multiple retries or fallback to `patch_hunk`.

---

## TUI Slash Commands

```text
/auto             Let Hematite choose the narrowest effective workflow
/ask [prompt]     Sticky read-only analysis mode; optional inline prompt
/code [prompt]    Sticky implementation mode; optional inline prompt
/architect [prompt]  Sticky plan-first mode; optional inline prompt that can refresh `.hematite/PLAN.md`
/read-only [prompt]  Sticky hard read-only mode; optional inline prompt
/gemma-native [auto|on|off|status]  Auto/force/disable Gemma 4 native formatting
/new              Reset session and clear context
/forget           Purge saved conversation memory and wipe visible session state
/vein-reset       Wipe the RAG index; rebuilds automatically on next turn
/think            Enable Gemma-4 native reasoning channel
/no_think         Enable lower-effort reasoning
/lsp              Start language servers manually
/worktree list    List all git worktrees
/worktree add <path> [branch]  Create isolated worktree
/worktree remove <path>        Remove a worktree
/worktree prune   Remove stale worktree entries
/swarm <directive>  Spawn parallel worker agents
/diff             Show git diff --stat
/copy             Copy the session transcript
/undo             Undo last file edit
/clear            Clear visible dialogue and side-panel session state
/help             Show all commands
```

Workflow note:

- `/ask` is for explanation and repo understanding without mutation
- `/code` is for implementation work and should consume the current plan handoff when one exists
- `/architect` is for planning and solution design before editing, and can persist a reusable implementation handoff
- `/read-only` is the hard no-mutation workflow
- `/auto` returns Hematite to normal behavior
- each of those workflow commands can also take an inline prompt, for example `/ask why is this failing?` or `/code fix the startup banner`

**Hotkeys:** `Ctrl+B` brief mode, `Ctrl+P` professional mode, `Ctrl+Y` approvals off, `Ctrl+T` voice toggle, `Ctrl+Z` undo, `Ctrl+Q`/`Ctrl+C` quit, `ESC` cancel

---

## Voice

No setup required. Voice is built in.

The full Kokoro TTS engine — model weights, all 54 voices, and ONNX Runtime 1.24.2 — is baked into the Hematite binary at compile time. Press `Ctrl+T` to toggle it on.

**Why this matters:** most local TTS tools require a separate Python runtime, manual model downloads, or specific system DLL versions. Hematite ships everything in one binary. The only runtime dependency is `DirectML.dll`, which is bundled in the portable release and ships with Windows 10 1903+.

**54 voices** across American English, British English, Spanish, French, Hindi, Italian, Japanese, and Chinese. Recommended: `af_heart`, `af_bella`, `am_michael`, `bf_emma`, `bm_george`.

```
/voice           list all voices with numbers
/voice 17        select voice #17
/voice am_adam   select by ID
```

Voice speed and volume are configurable in `.hematite/settings.json`:

```json
"voice": "am_michael",
"voice_speed": 1.1,
"voice_volume": 1.0
```

---

## Configuration

Hematite reads `.hematite/settings.json` from your project root:

```json
{
  "mode": "developer",
  "trust": {
    "allow": ["."],
    "deny": []
  },
  "api_url": "http://localhost:1234/v1",
  "context_hint": "This is a Rust project using Axum and SQLx.",
  "fast_model": "gemma-4-9b",
  "think_model": "gemma-4-27b",
  "gemma_native_auto": true,
  "gemma_native_formatting": false,
  "verify": {
    "default_profile": "rust",
    "profiles": {
      "rust": {
        "build": "cargo build --color never",
        "test": "cargo test --color never",
        "lint": "cargo clippy --all-targets --all-features -- -D warnings",
        "fix": "cargo fmt",
        "timeout_secs": 120
      }
    }
  }
}
```

**`api_url`** — LLM provider endpoint. Defaults to `http://localhost:1234/v1` (LM Studio). Set this to point Hematite at any OpenAI-compatible server:

| Provider | `api_url` |
|---|---|
| LM Studio (default) | `http://localhost:1234/v1` |
| Ollama | `http://localhost:11434/v1` |
| Remote machine | `http://192.168.x.x:1234/v1` |

This overrides the `--url` CLI flag. The value is the `/v1` base path — Hematite appends `/chat/completions`, `/models`, and `/embeddings` automatically.

**`context_hint`** — an optional string injected into the system prompt every turn. Use it to give Hematite a permanent shortcut to the parts of your project it would otherwise have to rediscover by reading files.

The best hints tell the model **where things live** — large files with non-obvious structure, key entry points, or the pattern it needs to find a specific type of change:

```json
"context_hint": "Entry point is src/main.rs. Config loading is in src/config.rs."
"context_hint": "This is a Next.js 14 app using the App Router. Pages live in src/app/."
"context_hint": "Database schema is in db/schema.sql. ORM models are in src/models/."
"context_hint": "UI commands are handled in src/ui/tui.rs around line 950 — grep for '/copy' to find the insertion point."
"context_hint": "API routes are in src/routes/. Middleware is in src/middleware/auth.rs."
```

**When it pays off:** large files (500+ lines) where the model would otherwise read top-to-bottom to find a target. One good hint on a big file can save 3-5 wasted read_file calls per turn.

**Keep it short.** It's injected on every turn, so every token here costs on every request. One or two sentences is ideal. Leave it `null` if your project structure is self-evident from the file tree — Hematite's Vein RAG will handle most discovery on its own.

Permission modes: `read-only`, `developer`, `system-admin`

Workspace trust roots: `trust.allow` and `trust.deny`

`verify_build` prefers these per-project profiles for `build`, `test`, `lint`, and `fix`. If no profile is configured, Hematite falls back to stack-aware defaults where it can and tells you when a profile is needed for a less-standard action.

For Gemma 4 models, `gemma_native_auto: true` lets Hematite switch into the safer native-formatting path automatically at startup. Use `gemma_native_formatting: true` only when you want to force that path on explicitly, and use `/gemma-native off` if you need to disable it for troubleshooting.

By default, Hematite trusts the current workspace root (`"."`) for normal destructive-tool posture. You can add extra allowlisted roots or explicitly denied roots in `.hematite/settings.json` when you want workspace trust to be broader or stricter.

---

## MCP Servers

Hematite can load external MCP tools from JSON config files at:

- global: `~/.hematite/mcp_servers.json`
- workspace: `.hematite/mcp_servers.json`

Workspace entries override global entries with the same server name. The current runtime supports **stdio MCP servers** and reloads the config on each turn, so you can add or edit servers without restarting Hematite.

On Windows, wrapper commands such as `npx`, `npm`, `.cmd`, and `.bat` launchers are resolved automatically, so standard MCP examples work without hardcoding absolute paths.

Hematite captures MCP server stderr in memory instead of letting child processes write directly into the TUI. That avoids screen corruption in terminals like VS Code and does not create extra MCP log files on disk.

Example:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "docs": {
      "command": "uvx",
      "args": ["your-mcp-server"],
      "env": {
        "API_KEY": "set-me"
      }
    }
  }
}
```

When a server starts successfully, its tools are exposed to the model with names like `mcp__filesystem__read_file`. Failed servers are skipped.

---

## The Rusty Companion (Optional)

Run with `--rusty` to enable the Rusty personality system. Your companion is deterministically generated from your machine ID, so the same hardware always produces the same Rusty. Chaos and snark levels affect response tone. `--stats` prints the profile and exits.

---

## Cleanup

```powershell
pwsh ./clean.ps1
pwsh ./clean.ps1 -Deep
```

This removes ghost backups, scratch diffs, Hematite logs, runtime session files, temporary files, and old generated reports. Voice assets, settings, MCP config, and the Vein RAG database are preserved. Use `-Deep` if you also want to remove build artifacts (`target/`, `onnx_lib/`) and the Vein index (`vein.db`).

---

*Built for Windows. Runs on your hardware. Nobody else sees your code.*
