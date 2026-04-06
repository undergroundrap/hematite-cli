# Hematite CLI Documentation

## What this project is

Hematite is a local AI coding harness built in Rust. It runs on your machine, uses LM Studio as the local model runtime on `localhost:1234`, and is tuned for Gemma-4 and other Gemma-family models. The terminal TUI is one interface layer of the product, not the whole product. The main engineering target is a single-GPU consumer Windows setup, especially RTX 4070-class hardware.

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

## Hotkeys and Commands

- `ESC`: cancel the current task and copy the session transcript to the clipboard
- `Ctrl+Q` / `Ctrl+C`: exit Hematite and copy the session transcript
- `Ctrl+T`: toggle voice
- `/copy`: copy the session transcript manually
- `/clear`: clear visible dialogue and side-panel session state
- `/forget`: purge saved conversation memory and wipe visible session state
- `/swarm`: trigger parallel worker agents

Requires LM Studio running locally with a model loaded and the server started on port `1234`.

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
    vein.rs             SQLite FTS local retrieval.
    deep_reflect.rs     Idle-triggered session memory synthesis.
libs/
  kokoros/              Vendored voice synthesis library.
```

## Key Concepts

- `InferenceEvent`: the enum flowing from agent to TUI over `mpsc`
- Thought routing: model reasoning is routed to the side panel instead of the main chat
- `SPECULAR` panel: shows live reasoning, recent reasoning trace, and watcher events
- `ACTIVE CONTEXT`: shows the current working file set
- Ghost system: `.hematite/ghost/` stores pre-edit backups
- Hardware guard: `gpu_monitor.rs` watches VRAM and can force brief mode or reduce swarm fanout

## Model Behavior Notes

- Some local models omit an opening reasoning tag; the streamer handles this
- Some local servers return `tool_calls: []` instead of `null`; Hematite filters this
- Conversation history slices must start with a `user` message for LM Studio/Jinja alignment
- Tool hallucination guards block fake tool names such as `thought` or `reasoning`

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

- Exit and cancel flows copy the session transcript to the clipboard
- Session reports are written under `.hematite/reports/`

## Cleanup

```powershell
pwsh ./clean.ps1
pwsh ./clean.ps1 -Deep
```

This removes scratch files, logs, ghost backups, and runtime session artifacts. Deep cleanup also removes build outputs such as `target/` and `onnx_lib/`.
