# Hematite CLI Documentation

## What this project is

Hematite is a local AI coding harness built in Rust. It runs on your machine and uses any OpenAI-compatible local model server. The default target is LM Studio on `localhost:1234`, but the endpoint is configurable. The terminal TUI is one interface layer of the product, not the whole product. The main engineering target is a single-GPU consumer Windows setup, especially RTX 4070-class hardware.

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

## Hotkeys and Commands

- `ESC`: cancel the current task and copy the session transcript to the clipboard
- `Ctrl+Q` / `Ctrl+C`: exit Hematite and copy the session transcript
- `Ctrl+T`: toggle voice
- `/voice`: list all available TTS voices with numbers
- `/voice N` or `/voice <id>`: select a voice by number or ID — saves to `.hematite/settings.json` and takes effect immediately
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
- Startup greeting prints active endpoint (`Endpoint: http://localhost:1234/v1`) so misconfigured providers are immediately visible

## The Vein — Local RAG

The Vein is Hematite's retrieval-augmented generation layer. At the start of each turn it indexes
any changed files and queries for context relevant to the user's message. Results are injected into
the system prompt so the model starts with the right code already in view, reducing tool calls.

**Per-project database:** stored at `.hematite/vein.db` inside the workspace root. Each project
folder gets its own index. The Vein learns from files on disk, not from conversation content.

**Two retrieval modes, hybrid-merged:**

- **BM25** (always available) — SQLite FTS5 full-text search with Porter stemming. Fast, zero GPU
  cost, works even when LM Studio has no embedding model loaded.
- **Semantic** (optional, higher quality) — Calls `/v1/embeddings` on LM Studio to embed each chunk
  using `nomic-embed-text-v2` Q8_0. Understands synonyms and concept-level matches; finds "what
  renders on startup" even when no file uses the word "banner". Vectors are stored in SQLite so they
  survive restarts without re-embedding.

**To enable semantic search:** load `text-embedding-nomic-embed-text-v2` in LM Studio alongside
your main coding model. On an RTX 4070 this costs ~512 MB VRAM — both models fit comfortably.
Status bar shows `VN:SEM` (green) when active, `VN:FTS` (yellow) for BM25-only.

**Automatic backfill:** if the embedding model is loaded after initial indexing, Hematite detects
unembedded chunks and fills them gradually (20 per turn) without needing a reset or file-touch.

**How hybrid ranking works:** semantic hits score 1.0–2.0 (preferred), BM25 fills to 0.0–1.0 for
paths not already covered. Results are deduplicated by file path and capped at 1500 chars total.

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
- `edit_file` and `multi_search_replace` normalize CRLF → LF before matching so model search strings (always LF) work correctly on Windows files

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

## Cleanup

```powershell
pwsh ./clean.ps1
pwsh ./clean.ps1 -Deep
```

This removes scratch files, logs, ghost backups, and runtime session artifacts. Deep cleanup also removes build outputs such as `target/` and `onnx_lib/`.
