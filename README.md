# hematite

**A local GPU coding agent harness and terminal CLI for LM Studio. Windows-first. Optimized for Gemma-4, compatible with Gemma-family models. No API key. No cloud. No per-token cost.**

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
- NVIDIA GPU with 8 GB+ VRAM recommended; 12 GB is a good target for larger Gemma variants
- Rust toolchain if building from source

**Recommended models:** any Gemma-family model that behaves well in LM Studio, including stock Gemma-4 checkpoints and custom Gemma variants. Hematite is most tuned for Gemma-4 style prompting.

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

# 4. Skip approval modals (YOLO mode)
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
| `map_project` | Recursive project structure map |
| `shell` | Run PowerShell commands with timeout and output capping |
| `git_commit` | Stage all and commit with Conventional Commits style |
| `git_push` | Push to origin HEAD |
| `git_worktree` | Create, list, prune, and remove isolated worktrees |
| `verify_build` | Auto-detect project type and run build validation |
| `clarify` | Ask the user a question when genuinely blocked |

---

## Key Features

### Gemma-4 Native Architecture

Hematite is aligned with the Gemma-4 E4B prompting model. It uses native control tokens such as `<|think|>`, `<|turn>`, and `<|tool_call>`, and keeps internal reasoning in a dedicated TUI panel instead of polluting the main chat.

### Hardware-Aware Context Management

Hematite reads GPU VRAM every 2 seconds. When memory pressure rises, it compacts earlier and caps parallel workers. The loaded model's context window is detected from LM Studio and injected into the system prompt so the model knows its own budget.

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

### Background Audio Engine

Press `Ctrl+T` to enable real-time text-to-speech. Hematite uses a statically linked 24 kHz Kokoro pipeline and loads the voice model in the background so the CLI can stay responsive during startup.

---

## TUI Slash Commands

```text
/new              Reset session and clear context
/forget           Purge saved conversation memory and wipe visible session state
/think            Enable Gemma-4 native reasoning channel
/no_think         Enable lower-effort reasoning
/worktree list    List all git worktrees
/worktree add <path> [branch]  Create isolated worktree
/worktree remove <path>        Remove a worktree
/worktree prune   Remove stale worktree entries
/swarm <directive>  Spawn parallel worker agents
/diff             Show git diff --stat
/undo             Undo last file edit
/clear            Clear visible dialogue and side-panel session state
/help             Show all commands
```

**Hotkeys:** `Ctrl+B` brief mode, `Ctrl+P` professional mode, `Ctrl+Y` YOLO mode, `Ctrl+T` voice toggle, `Ctrl+Z` undo, `Ctrl+Q` quit, `ESC` cancel

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
  "think_model": "gemma-4-27b"
}
```

Permission modes: `read-only`, `developer`, `system-admin`

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
