# Hematite Architecture

This is the contributor map of Hematite's current runtime boundaries.

The goal is simple:

- `conversation.rs` should orchestrate turns
- specialized modules should own their own policy or explanation logic
- the TUI should render runtime truth, not invent it
- LM Studio should remain the local model runtime, not the product brain
- recurring workflow structure should live in the harness when local models are not dependable enough to infer it every turn

## Top-Level Entry Points

### `src/main.rs`

CLI entry point. Parses flags (`--no-splash`, `--yolo`, `--rusty`, `--brief`, `--stats`), then delegates to `src/runtime.rs`.

### `src/runtime.rs`

Owns runtime assembly and startup.

- builds the typed runtime bundle
- wires agent channels, watcher channels, voice, swarm, and LM Studio profile sync
- resolves the workspace root, loads config, launches the Vein, and spawns the agent loop
- handles the CWD guard (relocates to home if launched from an inaccessible system path)

If you are changing startup ownership, channel plumbing, or steady-state runtime boot, start here.

### `src/lib.rs`

Public crate surface for integration testing and diagnostics.

---

## Agent Layer — `src/agent/`

### `src/agent/conversation.rs`

Owns turn orchestration. The main loop of Hematite.

- handles user turns and slash-command flow
- assembles prompts via `prompt.rs`
- applies workflow mode policy (`/ask`, `/code`, `/architect`, `/read-only`, `/teach`, `/chat`)
- runs harness pre-run orchestration (multi-topic `inspect_host` before the model turn)
- injects `loop_intervention` for computation routing, shell-block recovery, Deno parse error recovery, and repeat-tool guards
- coordinates tool execution, verification, compaction, recovery, and final output
- drives session persistence via `save_session()` / `load_checkpoint()`

This file should not drift back into being a tool registry, product-truth catalog, or giant policy dump.

### `src/agent/routing.rs`

Owns query intent classification.

- classifies stable product-truth questions (identity, capability, mode questions)
- identifies routing classes: architecture, runtime diagnosis, computation sandbox, toolchain questions
- `needs_computation_sandbox()` — detects math/hash/financial/statistical/date queries and triggers pre-turn nudge toward `run_code`
- keeps prompt-shaped routing logic out of the main turn loop

### `src/agent/direct_answers.rs`

Owns stable product-truth responses (zero model tokens).

- identity and authorship answers
- workflow mode explanations
- Gemma-native settings explanations
- session memory policy
- recovery-recipe explanations
- MCP lifecycle explanations
- tool-class and tool-registry explanations

If a behavior is stable product truth and should answer with `Tokens: 0`, it belongs here.

### `src/agent/prompt.rs`

Owns system prompt assembly.

- builds the full system prompt per turn from workspace mode, Vein results, repo map, hot files, session memory, and workspace profile
- injects per-project rules from instruction files
- assembles the L1 hot-files context block and PageRank repo map injection
- manages workspace mode detection (Coding vs. Document)

### `src/agent/inference.rs`

Owns the model and tool protocol surfaces.

- `InferenceEngine` — HTTP client to LM Studio, streaming, tool calls
- chat message types (`ChatMessage`, role handling)
- `InferenceEvent` — the enum flowing from agent to TUI over `mpsc`
- tool definitions and tool metadata (`ToolMeta`)
- provider/runtime event flow (typed provider states: live, recovering, degraded, context-ceiling)
- prompt preflight and LM Studio profile sync (`loaded_context_length`)
- Gemma 4 native markup wrapping (controlled by `gemma_native_auto`)

Tool metadata should continue to live here or in adjacent registry-owned code, not leak back into ad hoc name lists.

### `src/agent/tool_registry.rs`

Owns the built-in tool catalog and dispatch.

- built-in tool definitions
- builtin dispatch path

`conversation.rs` should consume the registry, not act like a second registry.

### `src/agent/architecture_summary.rs`

Owns architecture-overview shaping.

- project-map and runtime-trace summary shaping
- architecture-overview assembly for grounded architecture questions
- read-only architecture batch pruning (prevents redundant tool calls on repeated architecture questions)

### `src/agent/parser.rs`

Owns Swarm workload parsing.

- resilient XML-ish parser for LLM output (handles trailing commas, broken escapes)
- maps model output to typed `WorkerTask` and `Hunk` structs for swarm dispatch

### `src/agent/swarm.rs`

Owns parallel worker agent coordination.

- `SwarmCoordinator` — spawns parallel worker agents for multi-file or multi-step tasks
- dispatches worker tasks from `parser.rs` output
- collects results and routes them to the diff review modal
- triggered via `/swarm`

### `src/agent/specular.rs`

Owns the SPECULAR panel event source.

- filesystem watcher via `notify` — fires events when workspace files change
- emits watcher events and shell-line events to the TUI SPECULAR panel
- provides side-panel content for reasoning traces and live activity

### `src/agent/compaction.rs`

Owns compaction and session carry-forward.

- recursive summary compression when context pressure mounts
- compaction thresholds and deduplicated summary normalization
- typed session ledger: checkpoint, blocker, recovery step, verification result, compaction metadata carry-forward
- budgeted recursive summaries clamped to real line/character limits

### `src/agent/recovery_recipes.rs`

Owns typed runtime recovery planning.

- named recovery scenarios: provider degraded, context window, prompt-budget pressure, history pressure, MCP workspace read blocked, proof-before-edit blockers
- recovery plans are explicit runtime policy, not buried in ad hoc branches

### `src/agent/policy.rs`

Owns tool-policy helper logic.

- destructive-tool classification
- path normalization
- MCP mutation/read helper checks
- target-path extraction

Keeps low-level policy helpers out of `conversation.rs`.

### `src/agent/permission_enforcer.rs`

Owns typed authorization decisions.

- outcomes: `Allow`, `Ask`, `Deny`
- inputs: workflow mode, workspace trust, shell rules, trust sensitivity, tool metadata

### `src/agent/trust_resolver.rs`

Owns workspace trust state.

- states: trusted, require-approval, denied
- trust affects destructive or external actions, not normal repo reads

### `src/agent/mcp.rs`

Owns MCP transport and framing.

- stdio MCP transport
- newline-delimited and `Content-Length`-framed protocol support
- TUI-safe process handling (MCP stderr captured in memory)

### `src/agent/mcp_manager.rs`

Owns MCP server lifecycle and discovery.

- loads `mcp_servers.json` from workspace and global scope
- typed MCP lifecycle states: unconfigured, healthy, degraded, failed
- resolves Windows launcher wrappers (`npx`, `.cmd`, `.bat`)

### `src/agent/lsp/`

Owns LSP server integration.

- `lsp/client.rs` — LSP protocol client
- `lsp/manager.rs` — language server process lifecycle and diagnostics collection
- surfaced via `/lsp` in the TUI

### `src/agent/config.rs`

Owns runtime config loading.

- loads and merges workspace `.hematite/settings.json` and global `~/.hematite/settings.json`
- workspace values win; global fills missing fields
- covers `api_url`, `model`, `voice`, `gemma_native_auto`, context settings, and verify-build profiles

### `src/agent/economics.rs`

Owns session economics tracking.

- tracks token usage and tool calls per session
- feeds the session report written on exit/cancel

### `src/agent/pricing.rs`

Owns model pricing tiers.

- USD-per-million-token cost table for known models
- used by economics to estimate session cost

### `src/agent/git.rs`

Git helpers used by the agent layer (branch, status, short log).

### `src/agent/git_context.rs`

Reads a short git status summary (branch + changes) injected into context at turn start.

### `src/agent/git_monitor.rs`

Background git state monitor — tracks uncommitted changes via atomic flags so the TUI badge stays current without blocking the turn loop.

### `src/agent/hooks.rs`

Owns hook configuration and dispatch.

- loads hook definitions from `.hematite/hooks.json`
- fires pre/post tool hooks as shell commands

### `src/agent/instructions.rs`

Owns project instruction discovery.

- walks up from the workspace root looking for instruction files (`.hematite/AGENTS.md`, `CLAUDE.md`, etc.)
- deduplicates and injects discovered rules into the system prompt

### `src/agent/transcript.rs`

Owns session transcript persistence.

- persistent transcript logger for the DeepReflect engine
- writes exchange pairs to `.hematite/reports/` for Vein indexing

### `src/agent/workspace_profile.rs`

Owns the auto-generated workspace profile.

- detects stack, package managers, important folders, noise folders, and build/test suggestions
- written to `.hematite/workspace_profile.json` on startup
- injected into the prompt as lightweight repo grounding
- inspectable via `/workspace-profile`

### `src/agent/utils.rs`

Shared agent utilities (ANSI stripping, text normalization helpers).

---

## Memory Layer — `src/memory/`

### `src/memory/vein.rs`

Owns The Vein — local RAG memory engine.

- SQLite FTS5 BM25 full-text retrieval (always available, zero GPU cost)
- semantic embedding retrieval via `/v1/embeddings` (optional, requires embedding model in LM Studio)
- hybrid ranking: semantic hits score 1.0–2.0, BM25 fills to 0.0–1.0
- indexes project source files, `.hematite/docs/`, `.hematite/reports/`, `.hematite/imports/`
- per-project database at `.hematite/vein.db`
- incremental indexing (only re-indexes changed files by mtime)
- active-room bias: tracks file edit heat and boosts retrieval toward the hot subsystem
- memory-type tagging: chunks tagged as `decision`, `problem`, `milestone`, `preference` for intent-matched retrieval
- status badge: `VN:SEM` (semantic active), `VN:FTS` (BM25 only), `VN:DOC` (docs-only outside a project)

### `src/memory/repo_map.rs`

Owns PageRank-powered repo maps.

- `tree-sitter` AST indexing across all source files
- `petgraph` PageRank to rank files by structural importance
- heat-weighted personalization: hottest files get score boosts so actively edited central files float to the top
- injected into the system prompt each turn so the model knows architecture without burning tool calls

### `src/memory/deep_reflect.rs`

Owns idle-triggered session memory synthesis.

- fires after 5 minutes of TUI inactivity
- reads the day's transcript and calls the local model to extract structured memories: files changed, decisions made, patterns observed, next steps
- outputs written to `.hematite/memories/<YYYY-MM-DD>.md`
- injected into the system prompt at startup as persistent session context

---

## Tools Layer — `src/tools/`

### `src/tools/mod.rs`

Tool registry and dispatch. Routes tool calls from the agent to the correct implementation.

### `src/tools/file_ops.rs`

File listing, reading, writing, and project mapping. Core file inspection tools.

### `src/tools/file_edit.rs`

Targeted editing helpers — `edit_file`, `patch_hunk`, `multi_search_replace`.

- CRLF→LF normalization before matching
- fuzzy match escalation: rstrip-only → full-strip → cross-file hint
- delta-corrected indentation on fuzzy matches

### `src/tools/shell.rs`

Shell execution.

- `execute_streaming` — streams each stdout/stderr line to the SPECULAR panel via `InferenceEvent::ShellLine`
- `execute_blocking` — blocking execution for background tasks
- blocked for computation tasks when `needs_computation_sandbox` fires (redirects to `run_code`)

### `src/tools/code_sandbox.rs`

Sandboxed code execution (`run_code` tool).

- Deno sandbox (JS/TS): `--deny-net --deny-env --deny-sys --deny-run --deny-ffi --allow-read/write=.`
- Python sandbox: blocked socket/subprocess/dangerous imports, clean environment
- Hard timeout: 10s default, up to 60s
- Deno detection order: LM Studio bundled copy → system PATH
- 16 KB output cap; scratch file overflow for large results

### `src/tools/host_inspect.rs`

SysAdmin and Network Admin inspection (`inspect_host` tool). 76+ read-only topics covering the full OS stack. See CLAUDE.md for the complete topic reference.

### `src/tools/guard.rs`

Safety checks for risky shell actions.

- blocks destructive commands
- whitelists safe read-only diagnostics (`get-counter`, `Get-Item`, `Test-Path`, `Select-Object`, `arp -a`, etc.)
- redirects structured diagnostic commands to `inspect_host` topics

### `src/tools/git.rs`

Git tool implementations — commit, push, branch, diff, log, worktree.

### `src/tools/git_onboarding.rs`

Git remote configuration helper — configures or updates a Git remote and optionally performs an initial push.

### `src/tools/verify_build.rs`

Build validation tool.

- runs per-project build, test, lint, and fix profiles from `.hematite/settings.json`
- falls back to stack autodetection
- exempt from the repeat-tool guard (fix-verify loops are legitimate)

### `src/tools/research.rs`

Web research tools.

- `research_web` — Jina Reader/Search for technical information
- `fetch_docs` — pulls external docs into readable form
- rate-limited; gracefully handles missing `JINA_API_KEY`

### `src/tools/runtime_trace.rs`

`trace_runtime_flow` — grounded read-only runtime/control-flow inspection. Gives the model a verified path for exact architecture questions instead of confident guessing.

### `src/tools/toolchain.rs`

`describe_toolchain` — verified read-only map of Hematite's built-in tools, when to use them, and what investigation order makes sense.

### `src/tools/vision.rs`

`vision_analyze` — base64-encodes images and passes them to the model via the multimodal vision path. Used for screenshot/diagram analysis.

### `src/tools/lsp.rs` / `src/tools/lsp_tools.rs`

LSP startup and language-aware tooling. Surfaces diagnostics from language servers.

### `src/tools/plan.rs`

`PlanHandoff` — persists architect session plans to `.hematite/PLAN.md` for `/architect` → `/implement-plan` handoff.

### `src/tools/tasks.rs`

`manage_tasks` — persistent TODO list for the agent in `.hematite/TASK.md`. Actions: list, add, update, remove.

### `src/tools/health.rs`

Quick workspace health check — file count, source structure summary. Surfaced via `/health`.

### `src/tools/repo_script.rs`

Workspace script runner — executes project-local build/test/lint/clean scripts with a hard timeout and output cap. Used by the workspace workflow lane.

### `src/tools/workspace_workflow.rs`

Structured workspace workflow invocations — rooted to the locked workspace root. Separate from Hematite's own maintainer scripts.

### `src/tools/scoping_tools.rs`

Context scoping helpers — `auto_pin_context` lets the model lock 1–3 core files into prioritized memory for the current turn.

### `src/tools/risk_evaluator.rs`

Swarm risk triage — classifies worker actions as LOW (auto-approve), MODERATE (log warning), or HIGH (Red Modal). Eliminates approval prompts for safe file reads while enforcing hard gates on destructive operations.

### `src/tools/tool.rs`

`RiskLevel` and shared tool types.

### `src/tools/tool_schema_cache.rs`

Static tool schema cache (`OnceLock`) — avoids reserializing tool definitions on every turn.

---

## UI Layer — `src/ui/`

### `src/ui/tui.rs`

Owns the operator interface.

- main transcript rendering (chat surface)
- SPECULAR panel: live reasoning traces, shell output lines, watcher events
- bottom status bar: LM Studio badge, VN badge, BUD/CTX meters, session error count
- runtime badges: provider health, compaction pressure, context ceiling
- approval prompts and diff preview modal (`Y`/`N`)
- voice toggle state
- `@` file autocomplete in the input field
- all slash-command UI flows

The TUI renders typed runtime truth from the agent/runtime layer. It is not the source of truth for provider health, recovery, or policy state.

### `src/ui/voice.rs`

Owns the self-contained TTS pipeline.

- Kokoro ONNX model (311 MB) and voice styles (27 MB) embedded via `include_bytes!`
- ONNX Runtime 1.24.2 statically linked — no system DLL dependency
- 54 voices across 7 languages
- `Ctrl+T` to toggle, `/voice` to switch, speed/volume configurable in `settings.json`
- 1024-token speech buffer so audio is never lost during ONNX graph optimization on first load

### `src/ui/gpu_monitor.rs`

Background VRAM polling.

- watches GPU memory usage and emits pressure signals
- can force brief mode or reduce swarm fanout under memory pressure

### `src/ui/modal_review.rs`

Swarm diff review modal — presents per-worker diffs for operator approval before applying.

### `src/ui/hatch.rs`

Companion soul/personality generation — `/reroll` hatches a new personality mid-session.

---

## Supporting Files

### `src/telemetry.rs`

Global session error counter (lightweight `Mutex<i32>`) — feeds the session error count badge in the status bar.

---

## Practical Rules For Contributors

- If you are adding a stable explanation, prefer `direct_answers.rs`.
- If you are adding a new routing class or computation-routing pattern, prefer `routing.rs`.
- If you are adding a low-level approval/path/MCP helper, prefer `policy.rs`.
- If you are changing the built-in tool list or builtin dispatch, prefer `tool_registry.rs`.
- If you are changing typed authorization behavior, prefer `permission_enforcer.rs` and `trust_resolver.rs`.
- If you are changing architecture-overview shaping, prefer `architecture_summary.rs`.
- If you are changing the live turn loop itself, use `conversation.rs`.
- If you are adding a new `inspect_host` topic, add it to `host_inspect.rs` and document it in CLAUDE.md.
- If you are adding a new slash command, wire it in `tui.rs` and document it in CLAUDE.md.

## What Not To Reintroduce

Avoid letting `conversation.rs` grow back into:

- a second tool registry
- a second policy registry
- a pile of direct-answer strings
- a second architecture summary formatter
- a home for dead compatibility wrappers

The product is strongest when each boundary owns one thing clearly.
