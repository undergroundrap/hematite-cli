# hematite

**A local GPU coding agent harness and terminal CLI for LM Studio. Windows-first. Built for single-GPU consumer hardware, especially RTX 4070-class machines. Optimized for Gemma-4, compatible with Gemma-family models. No API key. No cloud. No per-token cost.**

`hematite` is a high-performance local coding harness that turns LM Studio into a serious agentic CLI. Hematite owns the TUI, tool execution, file editing, git workflows, retrieval, voice, and orchestration layer. LM Studio handles model loading, swapping, and serving. Hematite is tuned around Gemma-4 behavior, but it can work with other Gemma-family models and custom Gemma variants you load through LM Studio.

[![Capabilities](https://img.shields.io/badge/HEM-CAPABILITIES-blueviolet)](CAPABILITIES.md)
[![Gemma-4](https://img.shields.io/badge/MODEL-GEMMA--4--E4B-blue)](https://hf.co/google/gemma-4-e4b-it)
![Windows](https://img.shields.io/badge/Windows-native-blue?style=flat-square)
![Linux](https://img.shields.io/badge/Linux-supported-green?style=flat-square)
![macOS](https://img.shields.io/badge/macOS-supported-lightgrey?style=flat-square)
![License](https://img.shields.io/badge/License-MIT-green?style=flat-square)

---

## Why Hematite

Most local AI coding tools are Linux-first afterthoughts that quietly fall apart on Windows. Hematite is built around the opposite assumption: the local coding agent should feel native on the machine you actually use.

- **Zero ongoing cost** - no API key, no subscription, no per-token billing
- **Complete privacy** - nothing leaves your machine
- **LM Studio native workflow** - easy local model loading, swapping, and updating
- **Gemma-tuned prompting** - built around Gemma-4 E4B control tokens and reasoning flow, while remaining usable with other Gemma-family checkpoints
- **Cross-platform shell correctness** - PowerShell on Windows, bash on Linux/macOS
- **4070-class local target** - designed around what a single consumer GPU can realistically sustain
- **GPU-aware harness** - reads VRAM live and adapts agent behavior
- **Offline after setup** - no cloud dependency once your local stack is in place

**Windows is the primary development target.** PowerShell integration, path handling, shell behavior, and sandbox isolation receive the most polish there. Linux and macOS are supported.

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

## Requirements

| Platform | Shell | GPU Monitoring |
|---|---|---|
| Windows 10/11 | PowerShell (`pwsh` / `powershell.exe`) | NVIDIA via `nvidia-smi` |
| Linux | bash | NVIDIA via `nvidia-smi` |
| macOS | bash / zsh | Degrades gracefully |

- [LM Studio](https://lmstudio.ai) with a model loaded and the local server running on port `1234`
- NVIDIA GPU with 8 GB+ VRAM recommended; 12 GB VRAM is the sweet spot Hematite is most actively shaped around
- Rust toolchain if building from source

**Recommended models:** any Gemma-family model that behaves well in LM Studio, including stock Gemma-4 checkpoints and custom Gemma variants. Hematite is most tuned for Gemma-4 style prompting.

**Primary hardware target:** a single RTX 4070-class GPU on a normal desktop Windows machine. Hematite is engineered around that constraint: limited local VRAM, one active consumer GPU, LM Studio as the serving layer, and open models that need strong tooling and context discipline instead of cloud-scale brute force.

---

## Quick Start

### Recommended User Path

1. Install LM Studio.
2. Load a model and start the local server on port `1234`.
3. Download a Hematite release bundle, or build Hematite from source.
4. Launch `hematite` from inside your project folder.

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

### Portable Release

1. Run `pwsh ./scripts/package-windows.ps1`.
2. Pick up the portable bundle from `dist/windows/Hematite-<version>-portable/`.
3. Run `hematite` inside any project directory.

### Installer Recommendation

If you want Hematite to be easy for normal users to adopt, ship both:

- a **portable zip** for power users
- a **Windows installer** that places `hematite.exe` on `PATH`, checks for LM Studio, and explains the first-run flow

The installer should present Hematite honestly as a **local GPU coding CLI for LM Studio**, not as the model runtime itself.

Current packaging commands:

```powershell
pwsh ./scripts/package-windows.ps1
pwsh ./scripts/package-windows.ps1 -Installer
```

Notes:
- the packaging script clears `CARGO_NET_OFFLINE` for the build step so ONNX Runtime provisioning does not fail in release mode
- `-Installer` requires Inno Setup (`iscc`) to be installed locally

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

Hematite gives the loaded model a real local tool suite for coding work:

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

### Gemma-4 Native Architecture

Hematite is aligned with the Gemma-4 E4B prompting model. It uses native control tokens such as `<|think|>`, `<|turn>`, and `<|tool_call>`, and keeps internal reasoning in a dedicated TUI panel instead of polluting the main chat.

### Hardware-Aware Context Management

Hematite reads GPU VRAM every 2 seconds. When memory pressure rises, it compacts earlier and caps parallel workers. The loaded model's context window is detected from LM Studio and injected into the system prompt so the model knows its own budget.

This is intentionally tuned around single-GPU consumer hardware. The design goal is not cloud parity; it is to get the best practical coding workflow out of a 4070-class local box.

### Adaptive Thought Efficiency

Using `--brief` or `/no_think`, Hematite injects low-effort reasoning instructions so simple tasks stay fast while deeper tasks can still use full thought depth.

### Recursive Compaction

When conversation history gets large, Hematite summary-chains older context instead of bluntly truncating it. The compaction threshold scales with context length and current VRAM usage.

### Ghost Commit System

Before every file edit, Hematite snapshots a hidden git ref at `refs/hematite/ghost`. `Ctrl+Z` in the TUI rolls back the last edit without touching branch history or the visible git log.

### Swarm Agents

`/swarm <directive>` spawns parallel worker agents that research, implement, and propose diffs for review. Worker count is capped automatically based on available VRAM.

### Project Rule Discovery

Drop a `CLAUDE.md` or `.hematite.md` in your project root. Hematite picks it up automatically and follows your project-specific coding standards every turn.

### The Vein (RAG Memory)

An SQLite FTS5 index of your project source is queried every turn so relevant code can be pulled into context without extra tool calls.

### Built-In Web Research

Hematite can search the web for technical information when local context is not enough. `research_web` is used to find likely documentation or debugging leads, and `fetch_docs` pulls the resulting pages into clean Markdown so the model can actually read them instead of guessing from snippets.

### Grounded Runtime Tracing

For architecture and control-flow questions, Hematite can use `trace_runtime_flow` to return a verified read-only runtime trace instead of relying on model memory alone. This is especially useful on local open models where exact symbol tracing is weaker than cloud frontier models.

### Grounded Tool Selection

For tooling-discipline and investigation-plan questions, Hematite can use `describe_toolchain` to return the real built-in tool surface, what each tool is good or bad for, and a concrete read-only investigation order. This helps local open models avoid inventing fake helper tools or fake symbol names when explaining how they would inspect a Rust codebase.

### Architecture-Aware Repo Mapping

`map_project` is not just a file tree. It returns a compact repo map with configuration markers, likely entrypoints, core owner files, and extracted top symbols so smaller local models can get useful spatial awareness before spending turns on deeper file reads or LSP queries.

### Architect -> Code Handoff

`/architect` can persist a compact implementation handoff into `.hematite/PLAN.md` and session memory. That handoff carries the goal, target files, ordered steps, verification action, risks, and open questions so `/code` can resume from a clean brief instead of reconstructing the plan from chat residue.

### Safe Gemma 4 Native Layer

Hematite now has a narrow Gemma 4 compatibility layer in the inference path. It does not rewrite the full conversation protocol. Instead, it detects Gemma 4 models and normalizes the specific malformed tool-argument patterns that local runs were actually producing, such as over-quoted `path` and `extension` fields or slash-delimited `grep_files` patterns.

### Vision Analysis

Hematite can inspect screenshots, diagrams, and UI captures through `vision_analyze`, which lets the model reason about visual bugs and interface state instead of relying only on text descriptions.

### Background Audio Engine

Press `Ctrl+T` to enable real-time text-to-speech. Hematite uses a statically linked 24 kHz Kokoro pipeline and loads the voice model in the background so the CLI can stay responsive during startup.

---

## TUI Slash Commands

```text
/auto             Let Hematite choose the narrowest effective workflow
/ask [prompt]     Sticky read-only analysis mode; optional inline prompt
/code [prompt]    Sticky implementation mode; optional inline prompt
/architect [prompt]  Sticky plan-first mode; optional inline prompt that can refresh `.hematite/PLAN.md`
/read-only [prompt]  Sticky hard read-only mode; optional inline prompt
/new              Reset session and clear context
/forget           Purge saved conversation memory and wipe visible session state
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

## Voice Setup (Optional)

To enable the Voice of Hematite, place the following assets in `.hematite/assets/voice/`:

- [kokoro-v1.0.onnx](https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx)
- [voices.bin](https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin) and rename it to `voices.bin`

Architecture notes:

- ORT and DirectML dependencies are staged next to the executable during build
- the 325 MB voice model loads in a background thread
- `misaki-rs` is used for English phoneme synthesis

Once assets are present, press `Ctrl+T` in the TUI to toggle voice.

---

## Configuration

Hematite reads `.hematite/settings.json` from your project root:

```json
{
  "mode": "developer",
  "context_hint": "This is a Rust project using Axum and SQLx.",
  "fast_model": "gemma-4-9b",
  "think_model": "gemma-4-27b",
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

Permission modes: `read-only`, `developer`, `system-admin`

`verify_build` prefers these per-project profiles for `build`, `test`, `lint`, and `fix`. If no profile is configured, Hematite falls back to stack-aware defaults where it can and tells you when a profile is needed for a less-standard action.

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
./clean.sh
# or, on Windows:
pwsh ./clean.ps1
```

This removes ghost backups, scratch diffs, Hematite logs, runtime session files, temporary files, and old generated reports. Voice assets, settings, MCP config, and the main RAG databases are preserved. Use `./clean.sh --deep` or `pwsh ./clean.ps1 -Deep` if you also want to remove build artifacts such as `target/` and `onnx_lib/`.

---

*Built for Windows. Runs on your hardware. Nobody else sees your code.*
