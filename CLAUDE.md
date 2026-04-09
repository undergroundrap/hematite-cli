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
- `Ctrl+O`: open file picker to attach a document (PDF/markdown/txt) for the next turn
- `Ctrl+I`: open file picker to attach an image for the next turn (vision path)
- `Ctrl+Z`: undo last file edit (ghost backup restore)
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
- Diff preview: before `edit_file`, `patch_hunk`, or `multi_search_replace` is applied, a coloured before/after diff modal is shown in the TUI; user presses Y to apply or N to skip; model is told "Edit declined by user." on N; bypassed in `--yolo` mode
- `read_file` satisfies the line-inspection grounding check so the model can go `read_file → edit_file` without a separate `inspect_lines` call

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
- Bump immediately before running `scripts/package-windows.ps1` — not after features land, not speculatively.
- Always use `bump-version.ps1` — never edit version strings by hand across files.
- After bumping, run `cargo build` (this also regenerates `Cargo.lock`), then commit `Cargo.toml Cargo.lock README.md CLAUDE.md` together as the bump commit, then run the release.

**Commit message for a version bump:**
```
chore: bump version to 0.2.0
```

## Release Build

**Step 1 — bump the version** (updates Cargo.toml, README.md, CLAUDE.md atomically):

```powershell
pwsh ./bump-version.ps1 -Version 0.2.0
```

Never edit version numbers by hand — they will drift across files.

**Step 2 — tag and push to trigger CI:**

```powershell
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin main
git push origin v0.2.0
```

Pushing the tag triggers `windows-release.yml` and `unix-release.yml` on GitHub Actions. Both workflows download the Kokoro voice model assets, run `cargo build --release`, package the artifacts, and attach them to the GitHub Release automatically when they go green. No manual upload needed.

**Local build (optional, for testing before tagging):**

```powershell
pwsh ./scripts/package-windows.ps1
```

- The ONNX model (311 MB) is baked into the binary at compile time — no separate download
- `DirectML.dll` is copied from `target/release/` automatically by the ORT build script
- Output: `dist/windows/Hematite-0.2.0-portable.zip` (~336 MB)
- `dist/` is gitignored — these are release artifacts, not tracked in source

## Cleanup

```powershell
pwsh ./clean.ps1
pwsh ./clean.ps1 -Deep
```

This removes scratch files, logs, ghost backups, and runtime session artifacts. Deep cleanup also removes build outputs such as `target/` and `onnx_lib/`.
