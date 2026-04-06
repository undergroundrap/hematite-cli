# Hematite Architecture

This is the short contributor map of Hematite's current runtime boundaries.

The goal is simple:

- `conversation.rs` should orchestrate turns
- specialized modules should own their own policy or explanation logic
- the TUI should render runtime truth, not invent it
- LM Studio should remain the local model runtime, not the product brain

## Core Boundaries

### `src/runtime.rs`

Owns runtime assembly.

- builds the runtime bundle
- wires agent channels, watcher channels, voice, swarm, and LM Studio profile sync
- spawns the agent loop

If you are changing startup ownership, channel plumbing, or steady-state runtime boot, start here.

### `src/agent/conversation.rs`

Owns turn orchestration.

- handles user turns and slash-command flow
- assembles prompts
- applies workflow mode policy during the turn
- coordinates tool execution, verification, compaction, recovery, and final output

This file should not drift back into being a tool registry, product-truth catalog, or giant policy dump.

### `src/agent/routing.rs`

Owns query intent classification.

- classifies stable product-truth questions
- identifies routing classes such as architecture, runtime diagnosis, capability questions, and toolchain questions
- keeps prompt-shaped routing logic out of the main turn loop

### `src/agent/direct_answers.rs`

Owns stable product-truth responses.

- identity
- workflow modes
- Gemma-native settings
- session memory policy
- recovery-recipe explanations
- MCP lifecycle explanations
- tool-class and tool-registry explanations

If a behavior is stable product truth and should answer with `Tokens: 0`, it likely belongs here.

### `src/agent/policy.rs`

Owns tool-policy helper logic.

- destructive-tool classification
- path normalization
- MCP mutation/read helper checks
- target-path extraction

This keeps low-level policy helpers out of `conversation.rs`.

### `src/agent/permission_enforcer.rs`

Owns typed authorization decisions.

- `Allow`
- `Ask`
- `Deny`

Inputs include workflow mode, workspace trust, shell rules, trust sensitivity, and tool metadata.

### `src/agent/trust_resolver.rs`

Owns workspace trust state.

- trusted
- require-approval
- denied

Trust is meant to affect destructive or external actions, not normal repo reads.

### `src/agent/recovery_recipes.rs`

Owns typed runtime recovery planning.

- provider degraded
- context window
- prompt-budget pressure
- history pressure
- MCP workspace read blocked
- proof-before-edit blockers

Recovery plans should be explicit runtime policy, not buried in ad hoc branches.

### `src/agent/compaction.rs`

Owns compaction and session carry-forward.

- recursive summary compression
- compaction thresholds
- typed session ledger
- checkpoint, blocker, recovery, verification, and compaction carry-forward

### `src/agent/tool_registry.rs`

Owns the built-in tool surface.

- built-in tool catalog
- builtin dispatch path

`conversation.rs` should consume the registry, not act like a second registry.

### `src/agent/architecture_summary.rs`

Owns architecture-overview shaping.

- project-map summary shaping
- runtime-trace summary shaping
- architecture-overview assembly
- read-only architecture batch pruning

### `src/agent/inference.rs`

Owns model/tool protocol surfaces.

- chat message types
- inference events
- tool definitions
- tool metadata
- provider/runtime event flow
- prompt preflight and LM Studio interaction

Tool metadata should continue to live here or in adjacent registry-owned code, not leak back into ad hoc name lists.

## Operator Surface

### `src/ui/tui.rs`

Owns the operator interface.

- main transcript rendering
- SPECULAR panel
- bottom status bar
- runtime badges
- approval prompts
- voice toggle state

The TUI should render typed runtime truth from the agent/runtime layer. It should not be the source of truth for provider health, recovery, or policy state.

## Practical Rules For Contributors

- If you are adding a stable explanation, prefer `direct_answers.rs`.
- If you are adding a new routing class, prefer `routing.rs`.
- If you are adding a low-level approval/path/MCP helper, prefer `policy.rs`.
- If you are changing the built-in tool list or builtin dispatch, prefer `tool_registry.rs`.
- If you are changing typed authorization behavior, prefer `permission_enforcer.rs` and `trust_resolver.rs`.
- If you are changing architecture-overview shaping, prefer `architecture_summary.rs`.
- If you are changing the live turn loop itself, use `conversation.rs`.

## What Not To Reintroduce

Avoid letting `conversation.rs` grow back into:

- a second tool registry
- a second policy registry
- a pile of direct-answer strings
- a second architecture summary formatter
- a home for dead compatibility wrappers

The product is strongest when each boundary owns one thing clearly.
