# Hematite Capabilities

This document summarizes the technical strengths of **Hematite-CLI** as a local GPU-aware coding harness for LM Studio and Gemma-family models, with the strongest optimization focus on single-GPU consumer hardware such as the RTX 4070 class.

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

## 4. Workspace-Native Tooling

Hematite is more than a chat shell around a local model.

- **File and shell tools**: direct project reading, editing, search, and shell execution
- **Architecture-aware repo mapping**: `map_project` now returns likely entrypoints, core owner files, and extracted top symbols instead of just a raw file tree
- **Git-aware workflows**: worktrees, commit helpers, and rollback via hidden ghost snapshots
- **Configurable verification**: `verify_build` can now use per-project build, test, lint, and fix profiles from `.hematite/settings.json` instead of relying only on stack autodetection
- **Project retrieval**: SQLite FTS-backed memory helps recover relevant local context each turn
- **Built-in web research**: `research_web` and `fetch_docs` let the harness search for technical information and pull external docs into a readable form when local context is insufficient
- **Grounded architecture tracing**: `trace_runtime_flow` gives the model a verified read-only path for exact runtime/control-flow questions instead of encouraging confident guessing
- **Grounded toolchain guidance**: `describe_toolchain` gives the model a verified read-only map of Hematite's actual built-in tools, when to use them, and what investigation order makes sense
- **Vision support**: screenshot and diagram analysis can flow through `vision_analyze` when a task benefits from visual inspection

## 5. Stateful Local Workflow

Hematite is built for repeated project use, not one-off prompts.

- **Lightweight session handoff**: Hematite carries forward compact task/project signal instead of replaying full chat residue by default
- **Architect -> code handoff**: `/architect` can persist a compact implementation brief in `.hematite/PLAN.md` and session memory so `/code` can resume from a structured plan
- **Safe Gemma 4 native layer**: Gemma 4 runs get narrow argument normalization for malformed tool calls without changing Hematite's broader conversation protocol
- **Session persistence**: active state is saved under `.hematite/`
- **Task awareness**: local task and planning files can shape agent behavior
- **Instruction discovery**: project rules are loaded automatically from workspace instruction files
- **Sticky workflow modes**: `/ask`, `/code`, `/architect`, `/read-only`, and `/auto` let the operator choose between analysis, implementation, plan-first, and hard read-only behavior

## 6. Voice and TUI Integration

Hematite includes built-in operator experience features that are part of the product, not bolted on later.

- **Integrated TUI**: dedicated chat, reasoning, status, and input surfaces
- **Embedded TTS path**: Kokoro-based voice runs locally with background loading
- **Live diagnostics**: runtime state, GPU load, and tool activity are surfaced during use

## 7. MCP Interoperability

Hematite can extend itself through external MCP servers without making MCP the core identity of the product.

- **Workspace and global MCP config**: discovers `mcp_servers.json` in both scopes
- **Windows launcher compatibility**: resolves `npx`, `.cmd`, and `.bat` wrappers correctly
- **Protocol resilience**: supports newline-delimited stdio and falls back to `Content-Length` framing
- **TUI-safe process handling**: MCP stderr is captured in memory so child processes do not corrupt the terminal UI

## 8. Local-First Product Boundary

Hematite is the **agent harness**. LM Studio is the **model runtime**.

That boundary gives Hematite three advantages:

- model swapping stays easy
- the harness stays focused on workflow quality
- local deployment remains simple for normal users

---

Hematite is strongest when treated as a polished local coding harness: GPU-aware, terminal-native, tool-rich, and tuned for serious project work on single-GPU consumer hardware, especially RTX 4070-class machines.
