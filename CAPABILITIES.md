# Hematite Capabilities

This document summarizes the technical strengths of **Hematite-CLI** as a local GPU-aware coding harness for LM Studio and Gemma-family models, with the strongest optimization focus on single-GPU consumer hardware such as the RTX 4070 class.

Hematite is not trying to be a generic cloud-agent platform in a terminal skin. Its product thesis is narrower and stronger:

- be the best **local coding harness for LM Studio**
- be honest about **consumer GPU limits**
- make **runtime truth, recovery, and repo grounding** visible to the operator
- turn open local models into a serious project-work tool instead of a chat wrapper

That is the lens for the capabilities below.

## What Makes It Distinct

- **Local runtime truth**: live model/context sync, prompt-budget pressure, compaction pressure, typed provider states, and recovery recipes are surfaced directly in the operator UI
- **Repo-grounded behavior**: Hematite prefers architecture tracing, repo mapping, tool discipline, and bounded inspection over freeform model improvisation
- **Single-GPU engineering**: context shaping, compaction, fallback prompting, and recovery are built around what a 4070-class machine can actually sustain
- **Windows-first local quality**: PowerShell behavior, path handling, packaging, and terminal ergonomics are treated as first-class product concerns
- **Agent-harness boundary**: LM Studio is the model runtime; Hematite owns the workflow, tooling, TUI, safety, retrieval, and orchestration layer

## 1. Model-Native Reasoning Flow

Hematite is built to preserve a separation between internal reasoning and user-facing output.

- **Reasoning channel support**: the inference layer parses model-native reasoning markers and keeps them out of the main chat transcript
- **Clean dialogue surface**: internal planning stays in the side panel instead of leaking into the main response
- **Tool-first workflow**: reasoning, tool calls, and final output follow a consistent turn structure

## 2. Precision Editing

Hematite is optimized for controlled code edits on large files.

- **Search-and-replace editing**: `multi_search_replace` requires exact local anchors instead of fragile absolute offsets
- **Failure over corruption**: malformed or weak matches are rejected rather than applied speculatively
- **Multi-hunk support**: disconnected edits can be applied safely in one turn without index drift

## 3. Hardware Awareness

Hematite continuously adapts to the machine it is running on.

- **VRAM monitoring**: live GPU usage is tracked so the harness can react before the session destabilizes
- **Adaptive brief mode**: output and worker behavior can tighten automatically under memory pressure
- **Single-GPU focus**: the runtime is shaped around one practical local GPU, not multi-GPU or cloud assumptions
- **4070-class target**: the design center is the common 12 GB consumer setup where open models need careful context shaping, compaction, and tool discipline
- **Live LM Studio context detection**: startup now prefers the loaded model's `loaded_context_length` from LM Studio so Hematite budgets against the active runtime context instead of an outdated fallback field
- **Live runtime-profile refresh**: before each turn, Hematite can resync the loaded LM Studio model ID and active context budget so model swaps or context changes do not require a full Hematite restart
- **Quiet background runtime sync**: while idle, Hematite can keep the status bar aligned with LM Studio's live model and CTX state and only emits a visible operator message when the runtime profile actually changes
- **Compact LM runtime badge**: the bottom status bar now exposes a low-noise LM Studio state badge so the operator can see live, stale, warning, or context-ceiling conditions at a glance
- **Provider-state machine**: retries and runtime failures emit compact provider states such as recovering, degraded, or context-ceiling so the operator can see what Hematite is doing without parsing long failure prose
- **Failure-state persistence**: a runtime refresh can update model and CTX without immediately clearing a real `LM:CEIL` or `LM:WARN` condition; those states persist until successful output proves recovery
- **Compaction-pressure meter**: the bottom bar now shows a compact percentage badge tied to Hematite's real adaptive compaction threshold so the operator can see when conversation history is approaching summary-chaining pressure
- **Prompt-budget meter**: the operator surface now exposes a separate `BUD:NN%` badge for total turn payload pressure against the live LM Studio context window, which catches small-context prompt blowups that are not visible from history compaction pressure alone
- **Tighter operator footer**: the input/status surface now prioritizes real controls and real signals, including a live session error count, instead of spending width on dead counters or unreliable terminal hints
- **Runtime-owned provider state**: recovery, degraded, live, and context-ceiling transitions are now emitted by the runtime layer itself instead of being guessed by the TUI from rendered tokens or error strings
- **Typed operator checkpoints**: SPECULAR now receives explicit runtime checkpoint states for provider recovery, prompt-budget reduction, history compaction, blocked policy paths, blocked recent-file-evidence edits, blocked exact-line-window edits, and other recovery/blocker transitions
- **Typed recovery recipes**: retries, runtime refreshes, prompt-budget reduction, history compaction, and proof-before-edit recovery are now described by named recovery scenarios and compact step recipes instead of only ad hoc branch logic
- **Runtime bundle boundary**: startup assembly for engine, channels, watcher, voice, swarm, and LM Studio profile sync now lives behind a typed runtime bundle instead of being hand-wired directly in `main.rs`
- **Typed permission enforcement**: tool authorization now converges through one runtime decision layer for allow, ask, or deny outcomes instead of splitting shell rules, MCP approval defaults, safe-path bypasses, and shell-risk classification across ad hoc branches
- **Workspace trust state**: the current repo root is resolved through a typed trust policy, so destructive or external actions can behave differently in trusted, unknown, or explicitly denied workspaces
- **Registry-owned tool metadata**: repo reads, repo writes, git tools, verification tools, architecture tools, workflow helpers, research tools, vision tools, and MCP tools now carry explicit runtime metadata so mutability, trust sensitivity, plan fit, and parallel-safe execution are less dependent on ad hoc name lists
- **Dedicated tool registry boundary**: built-in tool definitions and builtin-tool dispatch now live behind `src/agent/tool_registry.rs` so the conversation loop owns less catalog/dispatch glue and more of the actual turn policy
- **Typed MCP lifecycle state**: MCP server availability is now surfaced as unconfigured, healthy, degraded, or failed runtime state so external-server issues do not hide inside tool refresh side effects
- **Intent-class routing**: stable product truth, runtime diagnosis, repository architecture, toolchain guidance, and capability questions now flow through one shared intent classifier instead of a long stack of isolated phrase gates
- **Typed session ledger**: compact carry-over now remembers the latest checkpoint, blocker, recovery step, verification result, and compaction metadata instead of preserving only task text and working-set hints
- **Tiny-context fallback profile**: when LM Studio serves a very small active context window, Hematite can switch to a slimmer system prompt so simple prompts still fit instead of immediately exhausting the budget
- **Manual runtime refresh**: `/runtime-refresh` lets the operator force an LM Studio profile resync on demand, and context-window failures trigger the same refresh path automatically

## 4. Workspace-Native Tooling

Hematite is more than a chat shell around a local model.

- **File and shell tools**: direct project reading, editing, search, and shell execution
- **PageRank-powered Repo Maps**: Native context injection leverages `tree-sitter` for AST indexing and `petgraph` PageRank to surface the most structurally important files first — the model wakes up already knowing the architecture without burning tool calls
- **Git-aware workflows**: worktrees, commit helpers, and rollback via hidden ghost snapshots
- **Configurable verification**: `verify_build` can now use per-project build, test, lint, and fix profiles from `.hematite/settings.json` instead of relying only on stack autodetection
- **Project retrieval**: SQLite FTS-backed memory helps recover relevant local context each turn
- **Built-in web research**: `research_web` and `fetch_docs` let the harness search for technical information and pull external docs into a readable form when local context is insufficient
- **Grounded architecture tracing**: `trace_runtime_flow` gives the model a verified read-only path for exact runtime/control-flow questions instead of encouraging confident guessing
- **Grounded architecture overviews**: broad read-only architecture questions now combine the AST injection with one authoritative `trace_runtime_flow` topic instead of drifting into long repo rewrites
- **Grounded toolchain guidance**: `describe_toolchain` gives the model a verified read-only map of Hematite's actual built-in tools, when to use them, and what investigation order makes sense
- **Vision support**: screenshot and diagram analysis can flow through `vision_analyze` when a task benefits from visual inspection

## 5. Stateful Local Workflow

Hematite is built for repeated project use, not one-off prompts.

- **Lightweight session handoff**: Hematite carries forward compact task/project signal instead of replaying full chat residue by default
- **Architect -> code handoff**: `/architect` can persist a compact implementation brief in `.hematite/PLAN.md` and session memory so `/code` can resume from a structured plan
- **Safe Gemma 4 native layer**: Gemma 4 runs get narrow argument normalization for malformed tool calls without changing Hematite's broader conversation protocol
- **Gemma numeric-arg hygiene**: float-shaped tool arguments like `limit: 50.0` or `context: 5.0` are normalized so bounded inspections stay bounded
- **Opt-in Gemma native formatting**: `.hematite/settings.json` can enable Gemma-native request shaping for Gemma 4 models without changing the default path for other models
- **Provider-side prompt preflight**: oversized requests can be blocked before they go to LM Studio, reducing silent near-ceiling hangs
- **Structured runtime failures**: degraded provider turns, context-window overruns, blocked tool calls, and repeated tool loops are surfaced as classified operator states instead of ad hoc error prose
- **One-shot provider recovery**: empty or degraded LM Studio turns get one automatic retry before Hematite escalates the structured failure
- **Streaming-path failure discipline**: plain text generations and startup flows now surface structured provider failures instead of raw stream errors or silent empty completions
- **LM Studio context-mismatch detection**: provider errors like `n_keep >= n_ctx` are classified as `context_window` failures so Hematite points at the real budget mismatch instead of mislabeling them as generic provider degradation
- **Budgeted recursive summaries**: compaction summaries are normalized, deduplicated, and clamped to a real line/character budget so recursive context carry-forward stays cheaper and more stable on small local contexts
- **Session persistence**: active state is saved under `.hematite/`
- **Task awareness**: local task and planning files can shape agent behavior
- **Instruction discovery**: project rules are loaded automatically from workspace instruction files
- **Sticky workflow modes**: `/ask`, `/code`, `/architect`, `/read-only`, and `/auto` let the operator choose between analysis, implementation, plan-first, and hard read-only behavior

## 6. Voice and TUI Integration

Hematite includes built-in operator experience features that are part of the product, not bolted on later.

- **Integrated TUI**: dedicated chat, reasoning, status, and input surfaces
- **Self-contained TTS**: Kokoro voice engine (311 MB model, 54 voices, ONNX Runtime 1.24.2) is statically linked into the binary — no install, no Python, no system DLL dependency; `Ctrl+T` to toggle, `/voice` to switch voices, speed/volume configurable in `settings.json`
- **Live diagnostics**: runtime state, GPU load, and tool activity are surfaced during use
- **Hybrid thinking**: non-Gemma models (Qwen etc.) automatically use `/think` mode so the model decides how much reasoning each turn needs without user intervention

## 7. Sandboxed Code Execution

Hematite can run code the model writes in a restricted subprocess — enabling real computation, not pattern-matched guesses from training data.

**Why this matters vs. LM Studio's built-in chat:** LM Studio's chat interface can discuss algorithms, write code snippets, and explain how Fibonacci works. It cannot run any of it. When you ask a local model "what's Fibonacci(20)?", it reaches into training data and gives you a plausible answer — which may be right, may be slightly wrong, and cannot be verified without running it yourself. Hematite closes that gap: the model writes the code, Hematite executes it in a zero-trust sandbox, and the real output comes back in the same turn.

**Proof of concept — SHA-256 hash via Web Crypto API:**

```
User: compute the SHA-256 hash of the string "Hematite"

→ run_code (javascript, Deno sandbox, crypto.subtle.digest)

94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c
```

That result cannot come from training data. SHA-256 is deterministic but not memorizable — no model can produce `94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c` without actually running a hash function. It is real cryptographic computation in a sandboxed Deno process, returned in one tool call. LM Studio's chat UI — regardless of which model is loaded — cannot do this.

- **`run_code` tool**: model writes JavaScript/TypeScript or Python, Hematite executes it and returns the actual output
- **Deno sandbox (JS/TS)**: `--deny-net`, `--deny-env`, `--deny-sys`, `--deny-run`, `--deny-ffi`, `--allow-read/write=.` — zero-trust permission model; no network, no filesystem escape, no native library calls
- **Python sandbox**: blocked socket, subprocess, and dangerous module imports; clean environment via `env_clear`
- **Hard timeout**: 10 seconds by default, model-configurable up to 60 seconds; process killed on expiry
- **Automatic Deno detection**: Hematite finds Deno automatically — checks `settings.json` override, `~/.deno/bin/`, WinGet package store, system PATH, then LM Studio's bundled copy as a last resort. If you have LM Studio installed, you likely already have Deno and JS/TS execution works out of the box with no extra setup
- **Real math and logic**: the model can verify algorithms, run calculations, test data transformations, and fix errors from actual output — not training-data approximations
- **Practical use cases**: check a sorting algorithm on a real dataset, verify a regex against real strings, compute checksums, generate test fixtures, run a quick proof — all without leaving the conversation

## 8. MCP Interoperability

Hematite can extend itself through external MCP servers without making MCP the core identity of the product.

- **Workspace and global MCP config**: discovers `mcp_servers.json` in both scopes
- **Windows launcher compatibility**: resolves `npx`, `.cmd`, and `.bat` wrappers correctly
- **Protocol resilience**: supports newline-delimited stdio and falls back to `Content-Length` framing
- **TUI-safe process handling**: MCP stderr is captured in memory so child processes do not corrupt the terminal UI

## 9. Local-First Product Boundary

Hematite is the **agent harness**. LM Studio is the **model runtime**.

That boundary gives Hematite three advantages:

- model swapping stays easy
- the harness stays focused on workflow quality
- local deployment remains simple for normal users

---

Hematite is strongest when treated as a polished local coding harness: GPU-aware, terminal-native, tool-rich, and tuned for serious project work on single-GPU consumer hardware, especially RTX 4070-class machines.
