# Hematite CLI — Documentation

## What this project is

Hematite is a high-performance local AI agent TUI built in Rust. It runs on local hardware via LM Studio on `localhost:1234`. The harness is tuned for Gemma-4 and other Gemma-family models. The TUI runs in the terminal via ratatui + crossterm.

## Build & run

```bash
cargo build                         # debug build
cargo run                           # run with defaults
cargo run -- --no-splash            # skip blocking splash for automation / testing
cargo run -- --rusty                # enable Rusty personality system
cargo run -- --yolo                 # skip high-risk approval modals
cargo run -- --brief                # force concise output mode
cargo run -- --stats                # print Rusty soul stats and exit
cargo run -- --swarm-size 5         # set max parallel agent workers
cargo run -- --fast-model <id>      # override model for simple tasks
cargo run -- --think-model <id>     # override model for complex tasks
./clean.sh                          # purge runtime artifacts, logs, scratch files, and ghost backups
```

### Hotkeys & Commands (TUI)
- `ESC`: Cancel current task + **Auto-Diagnostic** (copies transcript to clipboard).
- `Ctrl+Q` / `Ctrl+C`: Exit Hematite + **Auto-Diagnostic**.
- `Ctrl+T`: Toggle voice (TTS).
- `/copy`: Manually copy session transcript to clipboard.
- `/forget`: Purge conversation memory.
- `/swarm`: Trigger parallel agent workers.

Requires LM Studio running with a model loaded and the local server started on port 1234.

## MCP Configuration

Hematite loads stdio MCP servers from:

- `~/.hematite/mcp_servers.json`
- `.hematite/mcp_servers.json`

Workspace config overrides global config by server name. On Windows, wrapper launchers such as `npx`, `npm`, `.cmd`, and `.bat` are resolved automatically.

## API Configuration

Hematite uses **Jina Reader/Search** for web research. While it has a public rate-limited tier, providing an API key is recommended for stable performance:

1. Get a free key (10M tokens) at [jina.ai](https://jina.ai).
2. Set the environment variable:
   - **Windows (PowerShell)**: `$env:JINA_API_KEY = "your_key_here"`
   - **Linux/macOS**: `export JINA_API_KEY="your_key_here"`
3. Alternatively, create a `.env` file (ignored by Git) in the root directory: `JINA_API_KEY=your_key_here`

## Architecture

```
src/
  main.rs               Entry point. Wires channels, spawns tasks, launches TUI.
  agent/
    inference.rs        InferenceEngine: HTTP to LM Studio. stream_messages() / call_with_tools().
    conversation.rs     ConversationManager: agentic loop, tool dispatch, context pruning.
    agent_loop.rs       Core turn logic — tool call iteration, thought routing.
    swarm.rs            SwarmCoordinator: parallel worker agents.
    specular.rs         OS file watcher — detects .rs changes, triggers cargo check.
    mcp.rs / mcp_manager.rs  MCP server integration.
    prompt.rs           System prompt builder and workspace rule injection.
    parser.rs           Tool call parsing.
    transcript.rs       Turn history serialisation.
    git.rs              Git integration helpers.
  tools/
    tool.rs             ToolCall / ToolResult types.
    file_ops.rs         read_file, write_file, list_directory.
    file_edit.rs        patch_file (diff-based editing).
    shell.rs            run_shell (powershell execution).
    health.rs           health_check tool.
    lsp.rs              LSP diagnostics.
    guard.rs            High-risk tool detection → approval modal.
    risk_evaluator.rs   Rates tool call risk level.
    mod.rs              Tool registry + dispatch.
  ui/
    tui.rs              Main TUI loop (ratatui). App state, event handling, rendering.
    voice.rs            VoiceManager: Native Kokoro-82M TTS synthesis (rodio, ort, misaki-rs).
    gpu_monitor.rs      Background nvidia-smi poller → Arc<GpuState> for live VRAM gauge.
    modal_review.rs     Diff review modal for swarm proposals.
    hatch.rs            Rusty soul generation (deterministic species hash from machine ID).
    mod.rs
  memory/
    vein.rs             "The Vein" — SQLite RAG memory (rusqlite, indexed by file path).
    deep_reflect.rs     DeepReflect: idle-triggered reflection/summarisation engine.
    mod.rs
  telemetry.rs          Optional structured logging.
libs/
  kokoros/              Vendored Kokoro-82M inference library (ORT 2.0, ndarray 0.17).
```

## Precision Referencing (@ notation)

You can explicitly point Hematite to specific files by using the `@` symbol in your chat:
- `"/explain how @src/main.rs handles tool calls"`
- `"refactor @src/tools/research.rs to use a new API endpoint"`

When you use `@`, Hematite will automatically prioritize reading those files to ensure the highest accuracy.

## Global Mode & Safety

Hematite is designed to be safe for global usage (e.g., running on your Desktop or User root):

1.  **Sandbox Lock**: By default, Hematite is locked to its `workspace_root`. It cannot "climb" out of its current project directory unless explicitly moved.
2.  **System Blacklist**: Hematite is **strictly forbidden** from accessing sensitive OS areas. Attempted access to the following paths will be hard-blocked by the `guard.rs` system:
    - **Windows**: `C:\Windows`, `C:\Program Files`, `C:\$Recycle.Bin`, `System Volume Information`.
    - **Unix**: `/etc`, `/dev`, `/proc`, `/sys`, `/root`, `/var/log`.
3.  **Credential Guard**: Configuration files like `.ssh/`, `.aws/`, `.env`, and `credentials.json` are globally blacklisted and invisible to the agent.
4.  **Vigil Mode (Red Modal)**: All high-risk commands (file deletions, network calls, system operations) trigger a **Red Approval Modal**. The agent cannot execute these without your manual confirmation.

## Key concepts

**InferenceEvent** — the enum flowing from agent → TUI over mpsc:
`Token | Thought | ToolCallStart | ToolCallResult | ApprovalRequired | Done | Error`

**Thought routing** — Hematite parses model reasoning markers and routes them to the side panel. The streaming path sends reasoning as `Thought` until the reasoning section closes, then switches to `Token`. Non-streaming calls use `extract_think_block()` before broadcasting.

**SPECULAR panel** — right side of TUI, shows live reasoning from the current turn only. Cleared on each new user input. White text, markdown rendered. Auto-scrolls to bottom; manual scroll overrides it until the turn completes (Done event resets `specular_auto_scroll = true`).

**The Core** — main chat panel, left side. Hematite messages rendered with inline markdown. Color: `Color::Rgb(180, 90, 50)` (rust brown).

**Ghost system** — `.hematite/ghost/` stores `.bak` files before edits, tracked by `ledger.txt`. All ghost files are gitignored and cleaned by `clean.sh` / `clean.ps1`.

**Hardware guard** — `gpu_monitor.rs` polls `nvidia-smi` every 2s. If VRAM ratio > 85%, Brief Mode auto-enables and swarm workers are capped at 1.

## Model behaviour notes

- Some local models omit the opening reasoning tag — the streamer handles this by treating the reasoning span as `Thought` until it closes
- Some local servers return `tool_calls: []` instead of `null` when there are no tool calls — filter this: `tool_calls.filter(|c| !c.is_empty())`
- **Jinja Alignment**: Every conversation history slice MUST start with a `role: user` message to prevent 400 Bad Request errors.
- **Orphan Purge**: Aggressively discard `assistant` or `tool` messages at the start of pruned history to maintain alignment.
- Hallucination guard in `dispatch_tool` blocks tools named `thought`, `think`, `reasoning`
- Dedup set `called_this_turn: HashSet<String>` prevents duplicate tool calls in one turn

## Commit style

Conventional commits, lowercase:

```
feat: add X
fix: correct Y
refactor: restructure Z
chore: update deps / clean repo
docs: update README
```

## Commit Practice
- **When to commit**: Commit after every verified standalone change (Green Build).
- **Atomic Commits**: Prefer many small, reviewable commits over one large "giant" commit.
- **Conventional Style**: Always use `feat:`, `fix:`, `refactor:`, or `docs:` prefixes.
- **Completion Discipline**: Follow the `$ralph` pattern: verify the work → run tests → commit.
- **Safety**: The `git_commit` tool is HIGH_RISK and will always trigger a Red Modal for your final review.

## Session Economics & Reporting

Hematite tracks session costs and tool usage in real-time ($0.002/1K input, $0.006/1K output). 
- **Persistence**: Upon exiting, a JSON report is saved to `.hematite/reports/session_YYYYMMDD_HHMMSS.json`.
- **Diagnostics**: Every exit (intentional or via `ESC` cancel) automatically copies the full session transcript and economic data to your clipboard for debugging "stalled" runs.

## Cleanup

```bash
./clean.sh
```

Removes: ghost backups, scratch diffs, Hematite logs, runtime session files, tmp/, stray log files, and old economic reports. The main RAG databases are preserved. Use `./clean.sh --deep` if you also want to remove `target/` and `onnx_lib/`.
