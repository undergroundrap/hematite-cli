# Hematite CLI — Architecture Audit

**Auditor:** Claude (external architecture review)
**Date:** 2026-06-10
**Repo state:** `v0.14.1`, commit `4b7cccb`, 1,092 commits since 2026-04-04
**Scale:** ~405K lines of Rust (`src/` + `tests/`), 273 tool modules, 127 host-inspection topics, 288 model-visible tool definitions

This document is written for a principal Rust engineer to review and implement from. Every material claim cites a file and, where useful, a line range. Claims I could not fully verify are marked as such.

---

## 1. Executive Summary

### What the repo currently is

Hematite is a two-month-old, single-author, extremely high-velocity Rust codebase that contains:

1. **A genuinely deterministic, model-free IT diagnostics product.** `inspect_host` (`src/tools/host_inspect.rs`, 19,085 lines, 127 topic match arms) plus the headless CLI surface (`--report`, `--diagnose`, `--triage`, `--fix`, `--fix-all`, `--watch`, `--query`) and the deterministic in-TUI lane (`/triage`, `/fix`, `/inspect`, `/query` handled with zero model calls at `src/agent/conversation.rs:3091-3225`). This part is real, deep, and works without any model.
2. **An agent harness for local OpenAI-compatible models** built around a monolithic conversation loop (`src/agent/conversation.rs`, 15,041 lines; `run_turn` alone spans roughly lines 3082–11130), a 12,681-line keyword routing layer (`src/agent/routing.rs`), a 288-tool registry (`src/agent/tool_registry.rs`, 8,344 lines / ~500KB), a SQLite FTS5 + embeddings RAG layer (`src/memory/vein.rs`, 2,958 lines), and a ratatui TUI (`src/ui/tui.rs`, 6,493 lines).
3. **A read-only MCP stdio server** exposing exactly one tool (`inspect_host`) with a layered redaction pipeline (`src/agent/mcp_server.rs`, 434 lines).
4. **A fully local voice engine** with ONNX weights baked into release binaries (`src/ui/voice.rs:80-83`, `libs/kokoros/`).
5. **An enormous documentation surface**: `CLAUDE.md` (383KB), `README.md` (146KB), `CAPABILITIES.md` (198KB).

### What it is trying to become

The definitive local-first AI coding harness and natural-language sysadmin for a 9B model on RTX 4070-class hardware — compensating for model weakness with harness scaffolding, deterministic grounding, and honest failure states.

### Does the architecture match the mission?

**Partially — and there is one place where the arithmetic does not close.** The mission says the 9B model's context is the scarcest resource. Yet `ConversationManager` broadcasts the **entire 288-tool schema to the model on every turn** (`turn_tools = self.tools.clone()` at `src/agent/conversation.rs:8900`; only yolo/sovereign modes filter, and sovereign removes just two tools at 8892-8898). The string content of `get_tools()` alone is ~125K characters (~31K tokens by the project's own bytes/4 estimator in `src/agent/inference.rs:1063-1067`; the serialized JSON array is larger). Against the default 8,192-token context (`src/agent/inference.rs:337`) — or even the 16,384 "compact window" threshold (`inference.rs:226-231`) — the preflight check at `inference.rs:1123-1141` must reject every tool-bearing request unless the operator runs a very large context window, which is exactly what 12GB of VRAM cannot afford alongside 9B weights. The routing layer's 240 `needs_*` predicates already know which tool families a query needs, but they are used only to inject steering text (`conversation.rs:5084+`), never to subset the schema. **This is the single most important architectural gap in the project, and also the cheapest to fix, because the routing layer needed to fix it already exists.**

### Strongest parts of the design

- **The deterministic IT lane.** Slash commands and headless flags that run real inspections with zero model involvement are exactly what "don't pretend the model is smarter than it is" looks like in practice.
- **Failure honesty machinery.** `RuntimeFailureClass` (`conversation.rs:804-857`), operator checkpoints, recovery recipes (`src/agent/recovery_recipes.rs`), capped empty-response nudges (`conversation.rs:10217-10240`), and a hard 25-iteration / 4-consecutive-error termination (`conversation.rs:8684-8686`, `9643-9656`) show a consistent philosophy of visible failure over silent retry.
- **The MCP server's security posture.** One read-only tool, schema-field allow-listing (`mcp_server.rs:313-341`), per-topic policy blocks, fail-safe semantic redaction (raw data is never forwarded if the redaction model is unreachable, `mcp_server.rs:282-287`), and an audit trail. Small, auditable, correct trust boundary.
- **Vein's pragmatism.** SQLite FTS5 BM25 always works; embeddings are opportunistic; mtime-based incremental indexing; heat/room reranking (`vein.rs:838-875`). No vector-DB dependency. Right-sized for the target machine.
- **The prompt budget degradation ladder** (`conversation.rs:13211-13330`): summarize newest large tool outputs → collapse older tool results → trim long chat → drop middle messages → drop old user messages. The ordering is sensible and user-instruction-preserving.

### Weakest or riskiest parts

- **Tool schema broadcast vs. context budget** (above) — blocking.
- **`run_turn` is an ~8,000-line function** inside a 15,041-line file. It is effectively unreviewable and untestable as a unit; the harness's core correctness property lives in a place no contributor can safely modify.
- **CI does not compile, test, or lint the code.** `.github/workflows/ci.yml` runs only `cargo fmt --check`, `cargo audit`, and `cargo deny` — and only on tags or manual dispatch. The ~7,400 test functions in this repo have never gated a merge. The `justfile` claims `check` "mirrors CI exactly" (justfile:11-12) while running clippy and tests that CI does not run.
- **The Python "sandbox" is cosmetic.** `wrap_python` (`src/tools/code_sandbox.rs:143-196`) monkey-patches `__import__`, `os.system`, and `os.popen` inside the same interpreter. Any of `importlib.reload(os)`, `ctypes`, or restoring `builtins.__import__` defeats it, and the filesystem is intentionally left open. The module doc claims "no filesystem escape" (`code_sandbox.rs:4`). The Deno path is real sandboxing; the Python path is not, and the Scientific Mandate's credibility partially rests on it.
- **Swarm is aspirational, not safe.** One VRAM snapshot at dispatch time (`swarm.rs:89-90`), silent truncation of excess tasks (`take(max_workers)`, swarm.rs:104), silently swallowed worker errors (`if let Ok(res)` at swarm.rs:121), and a dead-code patch applicator (`#[allow(dead_code)]` at swarm.rs:174).

### Verdict

**Yes — this is on a credible engineering path, with one honest caveat.** The deterministic inspection layer, the MCP server, the failure-state philosophy, and the Vein design are real engineering aimed squarely at the stated mission. But the project currently ships a harness whose tool surface cannot fit its own target model's context window, validated by a CI pipeline that never runs its tests. Both are fixable without a rewrite, and the routing layer needed for the first fix already exists. Fix those two things and the architecture and the mission line up.

---

## 2. Current Architecture Map

### 2.1 Top-level layout

```
src/
  main.rs            5,944 lines — entry, headless dispatch (--report/--diagnose/--triage/--fix/...)
  lib.rs             3,339 lines — CliCockpit clap struct (413 flags, no subcommands), public exports
  runtime.rs         1,228 lines — runtime assembly, channel wiring, teleport handshake
  agent/             ~55 modules — conversation loop, routing, inference, prompt, MCP, swarm, redaction
  tools/             273 modules — pure-Rust toolkit + host_inspect + sandbox + file ops
  memory/            vein.rs (RAG), repo_map.rs (tree-sitter AST map), deep_reflect.rs
  ui/                tui.rs, voice.rs, gpu_monitor.rs, modal_review.rs, terminal.rs, ...
libs/kokoros/        vendored fork of the Kokoro TTS engine (published as hematite-kokoros 0.1.3)
tests/               6 integration files (~7,100 test fns)
evals/               manual prompt suites + CSV score template (human-scored, not automated)
```

### 2.2 Inference engine (`src/agent/inference.rs`, 2,050 lines)

`InferenceEngine` wraps a `provider` trait object (LM Studio / Ollama / generic OpenAI-compatible, `src/agent/provider.rs`) behind an `RwLock`, with a KV semaphore serializing calls (`inference.rs:861-864`). Key responsibilities:

- **`call_with_tools` (845-938):** acquires the semaphore, optionally rewrites messages for Gemma-native formatting, runs `preflight_chat_request` (1123), calls the provider, then post-processes: native tool-call extraction from text (`extract_native_tool_calls`, 1535), argument normalization (1671), think-block stripping (1246-1271).
- **Context accounting:** `estimate_serialized_tokens` = `serde_json::to_vec(...).len() / 4 + 1` (1063-1067). `estimate_prompt_pressure` (1102) sums message tokens + tool-schema tokens + 32, and reserves `context_length / 8` (capped at 4,096) for output (1097-1100). `preflight_chat_request` hard-errors with an honest `context_window_blocked` message when the estimate exceeds the window.
- **Adaptive system prompts:** tiny (<?), compact (≤16,384, `inference.rs:226-231`), and full variants (537-538). Note that the *prompt* adapts to small windows but the *tool schema array* does not.
- **Default runtime profile:** model context defaults to 8,192 (337) until `detect_context_length` succeeds against LM Studio's `/api/v0/models`.

### 2.3 Conversation loop (`src/agent/conversation.rs`, 15,041 lines)

`ConversationManager` (struct at 1597-1673) owns history, the engine, all 288 tools (`tools: get_tools()` at 2242), Vein, the swarm coordinator, the voice manager, LSP manager, pinned files, diff tracker, recovery context, and ~30 more fields. `run_turn` (3082) is the monolith:

1. **Deterministic lane** (3091-3225): `/triage`, `/health`, `/fix`, `/inspect`, `/query` are answered by direct topic routing + `inspect_host` with no model call. `/new`, `/forget` reset locally.
2. **Intent classification:** `classify_query_intent` (`routing.rs:4872`) sets mode flags (sovereign, capability, host-inspection, scaffold, etc.).
3. **Intervention chain** (~4900-8650): a long first-match-wins `if loop_intervention.is_none() && needs_X(...)` ladder (e.g. 5084, 5096, 5114, 5129, 5140, 5208) that injects one steering system message per turn pointing the model at the right native tool.
4. **The iteration loop** (8686+): `max_iters = 25` (8684), consecutive-error counters (8686, 9638-9656), repeat-call detection maps, read/grep dedup sets, mutation tracking, Vein context injection on first iteration only (8796-8827), prompt budget enforcement (8863-8881), tool subset selection (8886-8900 — full clone except yolo/sovereign), `call_with_tools`, failure classification and recovery (8930+), empty-response nudges capped at 2 (10217-10240), deterministic closeout fallback (10241+).
5. **Tool dispatch:** `dispatch_tool` (11138) → `dispatch_builtin_tool` (`tool_registry.rs:7954`), a flat ~250-arm match. Argument-repair shims fill missing `fix_plan` issues and `dns_lookup` names/types from the user prompt (11159-11270).
6. **Budget trimming:** `enforce_prompt_budget` (13211) targets 0.68 × context using the five-stage ladder described above. **It counts only messages** (`estimate_prompt_tokens`, 13169) — the tool schema that `preflight_chat_request` *does* count is invisible to the trimmer, so trimming can succeed and preflight can still hard-fail.

### 2.4 Routing (`src/agent/routing.rs`, 12,681 lines)

Pure-function keyword classification over lowercased input. Three tiers:

- **Intent classification:** `classify_query_intent` (4872-5304) → `QueryIntent` flags; `DirectAnswerKind` (16-36) short-circuits ~21 product/identity questions to canned grounded answers.
- **Host-topic routing:** `preferred_host_inspection_topic` (558-2813, ~2,255 lines of ordered if-chains) and `all_host_inspection_topics` (2814-4564) map natural language to the 127 inspection topics. Ordering is priority; collisions are managed by hand (see `tests/routing_precision.rs:7` "priority_collision_fix").
- **Tool steering:** ~240 `needs_*` predicates (5540 onward: `needs_http_request` 5664, `needs_docker_ops` 5743, `needs_csv_tools` 6126, …) consumed by the intervention chain in `conversation.rs`.

All matching is `contains`-based via helpers (57-63) plus one Aho-Corasick set for code keywords (99-109). English-only, no stemming, no fuzzy matching, no scoring — order and phrase choice are the precision mechanism.

### 2.5 Prompt assembly (`src/agent/prompt.rs`, 331 lines)

`SystemPromptBuilder::build` (88-330) concatenates: workspace-mode framing (Coding/Document/General detected at 17-73) → identity/tone → base instructions → global `~/.hematite/CLAUDE.md` (via `USERPROFILE` only, 137) → project guidance files (6KB cap each) → skill catalog → deep-context rules → dynamic section (compacted summary, session memory, environment, root file inventory, TASK/PLAN excerpts, MCP tool list) → a 19-rule "HEMATITE OPERATIONAL PROTOCOL" (308-327).

Notable hazards (detailed in §4): hardcoded `"- Operating System: Windows (User workspace)"` (239); stale "81+ diagnostic topics" (119) vs. the 127 real topics and the "128+" claim in all docs; rule 2 (310, "assume performance questions are about code") in direct tension with the thrice-repeated "Hardware Truth" mandate (100-113); rule 3 (311) instructs XML-tag tool calling while the standard path uses OpenAI-format tools.

### 2.6 Vein RAG (`src/memory/vein.rs`, 2,958 lines) and repo map (`src/memory/repo_map.rs`, 529 lines)

- **Storage:** SQLite with `chunks_fts` (FTS5/BM25), `chunks_vec` (768-dim embedding BLOBs), `chunks_meta`, `file_heat`. Embeddings are decoded once into an in-RAM cache at startup (466-507).
- **Indexing:** `index_project` (~900) walks the workspace (skips `target/`, `.git/`, `node_modules/`, `.hematite/`; 512KB file cap), chunks by language symbols (`chunk_by_symbols` 2816, `chunk_rust_symbols` 2832) or paragraphs/sliding windows (2910-2955), and incrementally re-indexes on mtime change. Also indexes `.hematite/docs/` (PDF/MD/TXT), session reports, and imported chats.
- **Retrieval:** `search_context` (838-875) gathers `limit*4` candidates from BM25 + semantic in parallel, reranks with query signals + "active room" heat bias (878-891), dedups by path, truncates to limit. Semantic results are score-boosted to win ties.
- **Concurrency:** `unsafe impl Send/Sync for Vein` (52-53) justified by an `Arc<Mutex<Connection>>` comment — sound today, fragile to future field additions.
- **Repo map:** tree-sitter queries (Rust/Python/TS/JS) extract symbols; petgraph builds a reference graph; output is a condensed AST layout injected as `repo_map` on the manager.

### 2.7 Host inspection (`src/tools/host_inspect.rs`, 19,085 lines)

`inspect_host` (13) dispatches 127 topic arms. Implementation is overwhelmingly PowerShell/CIM-based (324 `powershell`/`Get-CimInstance` references) under `#[cfg(target_os = "windows")]` (132 occurrences) with 103 `#[cfg(not(windows))]` branches — a meaningful fraction of which are stubs (e.g. `inspect_resource_load` returns "not yet implemented for this platform", 1686-1689), while others (kernel update checks ~4620, systemd failed units ~4672, hwmon temps ~4728) are real. Privilege-limited output is annotated rather than hidden (`annotate_privilege_limited_output`, 289). The deterministic `fix_plan` engine (712-1638) classifies issues and emits exact-command remediation plans without a model.

### 2.8 TUI (`src/ui/tui.rs`, 6,493 lines)

`App` (760) holds messages, sidebar mode, autocomplete, attachments, pending approvals, context files, SPECULAR panel state. `run_app` (3030) is the crossterm event loop; diff-review modals (`PendingApproval`, 507; `src/ui/modal_review.rs`) gate model-proposed edits behind Y/N; swarm review requests block on a oneshot channel back into the TUI (3469, 4241). GPU telemetry (`src/ui/gpu_monitor.rs`) feeds both the header and swarm throttling.

### 2.9 MCP server (`src/agent/mcp_server.rs`, 434 lines)

Newline-delimited JSON-RPC 2.0 over stdio, protocol `2024-11-05` (30). Exposes exactly one tool. Request path: parse → method match (`initialize`/`initialized`/`ping`/`tools/list`/`tools/call`) → `dispatch_tool_call` (184) → `sanitize_args` strips all non-allow-listed fields (313-341) → policy block check → `inspect_host` → tiered redaction (None/Regex/Semantic, 233-288) → audit entry (295-303). Semantic redaction failure returns an error, never raw data (282-287). No auth, no rate limiting, no session state — acceptable for stdio (the transport *is* the trust boundary) but a real gap if a TCP/SSE transport is ever added.

### 2.10 Voice (`src/ui/voice.rs`, 405 lines; `libs/kokoros/`)

Feature-gated (`embedded-voice-assets`): release builds `include_bytes!` the ~310MB Kokoro ONNX model and voices file (80-83) and initialize the TTS session eagerly on a dedicated 32MB-stack thread at `VoiceManager::new` (56-58) — i.e., at TUI startup, not first use. crates.io/source builds disable voice with a visible status message (61-69) rather than failing silently — good honesty. Cost: a ~355MB portable zip (`dist/windows/Hematite-0.10.0-portable.zip`) and startup-time ONNX session construction.

### 2.11 Swarm (`src/agent/swarm.rs`, 293 lines)

`SwarmCoordinator::dispatch_swarm` (79-171): reads VRAM ratio once; >85% switches to sequential execution; spawns up to `max_workers` tokio tasks that each run one `generate_task_worker` call, write the result to `.hematite/scratch/worker_N.diff`, and block on a human review oneshot. `apply_patches_descending` (175-274) — descending-order hunk application with model-mediated conflict resolution — is `#[allow(dead_code)]`. `Drop` wipes the scratchpad (277-293).

### 2.12 Build, packaging, CI

- `Cargo.toml`: single crate + vendored `hematite-kokoros`; release profile is `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. `rust-toolchain.toml` pins stable with the MSVC target.
- Packaging: `scripts/package-windows.ps1` (portable zip), `installer/` (Windows installer), `scripts/package-unix.sh`, `install-unix.sh`. Version discipline via `bump-version.ps1`, `scripts/verify-version-sync.ps1`, `scripts/verify-doc-sync.ps1` (parses the topic list out of `host_inspect.rs` and checks docs).
- CI (`.github/workflows/ci.yml`): triggers on **tags and manual dispatch only**; single job: `cargo fmt --check`, `cargo audit`, `cargo deny`. **No build, no clippy, no tests, no Windows runner in CI** (Windows appears only in `windows-release.yml`, which builds but does not test).

### 2.13 Tests

| Suite | Size | What it covers |
|---|---|---|
| `tests/math_tools.rs` | 36,925 lines / 4,725 tests | math_util correctness (golden-value style) |
| `tests/diagnostics.rs` | 41,355 lines / ~2,411 tests (447 async) | vein, workspace profiles, inspect_host smoke, teleport markers, task parsing, much more |
| `tests/routing_precision.rs` | 27 tests | topic-routing collisions and additions |
| `tests/scientific.rs` / `data_analysis.rs` / `debug_routing.rs` | 5 / 3 / 1 | thin |
| In-module `#[test]` | 252 across 38 `src/` files | edge_redact patterns, host_inspect helpers, repo_map, etc. |

The corpus is large and real — but `just test` omits `math_tools`, `scientific`, and `data_analysis` (justfile:26-30), and CI runs none of it.

### 2.14 CLI surface and headless mode

`CliCockpit` (`src/lib.rs:113`) is a **single flat clap struct with 413 flags** — no subcommands. Headless dispatch in `main.rs` (68+) handles version report, cwd guard (77-89), shell completions, MCP server, reports, diagnose, triage, fix/fix-all, watch/snapshot/timeline/audit, and a large data-analysis flag family (`--compute`, `--plot`, `--fourier`, `--cluster`, `--pca`, `--ode`, …). The deterministic headless product is substantial; its discoverability and testability at 413 flat flags is not (see §4.9).

---

## 3. Mission Alignment Audit

| # | Goal | Status | Evidence |
|---|---|---|---|
| 1 | Single-GPU consumer hardware as primary constraint | **Partially present / contradicted in one place** | KV semaphore (`inference.rs:861`), VRAM-aware swarm throttle (`swarm.rs:89-90`), compact/tiny prompt variants (`inference.rs:534-538`), GPU monitor. **But** 288-tool schema broadcast (`conversation.rs:8900`) ≈ 31K+ tokens vs. 8,192 default context (`inference.rs:337`) contradicts the constraint at the core of the loop. |
| 2 | Tight harness scaffolding for 9B limitations | **Present** | Intervention chain (`conversation.rs:5084+`), argument-repair shims (11159-11270), empty-response nudges (10217), sequential tool gating, plan-first scaffold interception (8656-8675), deterministic closeouts (10241). This is the repo's most distinctive engineering. |
| 3 | Honest operator-visible failure states | **Present, with gaps** | `RuntimeFailureClass` + operator checkpoints (804-1020), `context_window_blocked` preflight errors (`inference.rs:1133`). Gaps: swarm swallows worker errors (`swarm.rs:121`) and silently drops tasks beyond `max_workers` (104); many `let _ =` sends in the TUI/loop discard channel failures. |
| 4 | Grounded host inspection, no model guessing | **Present** | 127 deterministic topics; `/inspect`/`/query` run with zero model (`conversation.rs:3138-3225`); privilege annotation (`host_inspect.rs:289`); prompt mandates `inspect_host` over raw shell telemetry (`prompt.rs:100-101`). |
| 5 | Routing precision (query → right tool, not shell fallback) | **Partially present** | 240 `needs_*` predicates + topic router; but precision is encoded as *ordering of hand-written contains-chains* (`routing.rs:558-2813`), guarded by only 27 tests (`tests/routing_precision.rs`); `shell` remains in the schema every non-sovereign turn, so fallback prevention is steering-text-strength only. |
| 6 | Context pressure management | **Partially present / inconsistent accounting** | Five-stage budget ladder (`conversation.rs:13211`), compaction module (`compaction.rs`, 833 lines), `/compact`, `/budget`, per-turn `TurnBudget` ledger (`economics.rs`). **But** the ladder counts messages only (13169) while preflight counts messages + tools — the two disagree by the size of the entire tool schema. |
| 7 | Verifiable computation (Scientific Mandate, run_code) | **Partially present / partly misleading** | Mandate in prompt rule 19 (`prompt.rs:327`); Deno sandbox with scoped `--allow-read=. --allow-write=.` and no network (`code_sandbox.rs:56-77`) is real; 4,725 math golden tests. **But** the Python sandbox is bypassable monkey-patching (143-196) and the module's "no filesystem escape" claim (line 4) is false for Python. |
| 8 | Local RAG that improves relevance | **Present, quality unproven** | Hybrid BM25+semantic with heat/room reranking (`vein.rs:838-891`), symbol-aware chunking (2816+), incremental indexing. No retrieval-quality eval exists anywhere in `tests/` or `evals/` — relevance is asserted, not measured. |
| 9 | MCP server as serious integration surface | **Present, deliberately narrow** | One read-only tool, arg sanitization, policy file, fail-safe semantic redaction, audit log (`mcp_server.rs`). "Serious enterprise surface" claims (auth, rate limits, ACLs beyond topic blocks) are not yet true — see §4.7. Description claims "Works on Windows, Linux, and macOS" (349) while many topics are Windows-only — overstated. |
| 10 | Privacy/redaction | **Present** | Tier-1 regex pack with tests (`edge_redact.rs`, 232 lines: user paths, MACs, serials, hostnames, AWS keys), semantic tier with fail-safe, per-topic policy file (`redact_policy.rs`), audit trail (`redact_audit.rs`). One of the most credible subsystems. |
| 11 | Swarm parallel agents on constrained VRAM | **Risky / partly aspirational** | Single VRAM snapshot, no per-worker re-check, silent task truncation, swallowed errors, dead-code patch applicator (`swarm.rs:89-175`). Wired into TUI (3469, 4241) so it runs — but "VRAM-safe fanout" is one boolean, not an architecture. |
| 12 | Voice with zero cloud dependency | **Present** | Baked ONNX weights (`voice.rs:80-83`), all-Rust kokoros, explicit disabled-state messaging for source builds (61-69). Costs: ~355MB artifact, eager init at startup. |
| 13 | Pure-Rust developer toolkit | **Present, correctness partially verified** | 273 tool modules, flat dispatch (`tool_registry.rs:7954`), 4,725 math golden tests. Most non-math tool modules have far thinner coverage; `just test` doesn't even run the math suite. |
| 14 | Teleportation and workspace hygiene | **Present** | Teleport handshake (`runtime.rs`, `conversation.rs:2478-2484`), OS-shortcut-directory guard (`docs/ARCHITECTURE.md`, `file_ops.rs::is_os_shortcut_directory`), scratch wipe on drop (`swarm.rs:277-293`), ghost backups (`.hematite/ghost`). |
| 15 | CI/CD correctness across Windows and Linux | **Missing** | `ci.yml` = fmt + audit + deny, tags-only. No `cargo test`, no `cargo clippy`, no `cargo build`, no Windows test runner, no Linux test runner. The repo's largest single liability. |
| 16 | Versioning discipline and release hygiene | **Partially present** | 35 tags, `bump-version.ps1`, `verify-version-sync.ps1`, clean conventional-commit history. But releases are tagged from code CI never tested, and `justfile`'s "mirrors CI exactly" claim is false (justfile:11 vs. ci.yml). |
| 17 | Documentation accuracy / sync | **Partially present / drifting** | `verify-doc-sync.ps1` is real and parses source truth. But: `prompt.rs:119` says "81+ topics", `mcp_server.rs:7,349` say "116+", actual is 127, docs say "128+"; `conversation.rs:1239` describes a "Cli struct in src/main.rs" that actually lives in `lib.rs` as `CliCockpit`; README's "measured 42% reduction in context-melt regressions" (~line 441) has no reproducible benchmark in-repo; 383KB of CLAUDE.md cannot be kept honest by hand. |

---

## 4. Architecture Risks

Ordered by severity.

### 4.1 BLOCKING — Tool schema broadcast defeats the context budget

- **Where:** `src/agent/conversation.rs:8886-8900` (`turn_tools` = full clone), `src/agent/tool_registry.rs:19-7953` (288 `make_tool` definitions, ~125K chars of strings), `src/agent/inference.rs:337` (8,192 default context), `inference.rs:1102-1141` (pressure estimate + preflight).
- **Why it matters:** the entire mission is "make a 9B model's context work harder." By the project's own estimator, tool schemas alone consume ~31K+ tokens before a single message is added. At the default context, preflight must reject every tool call; at a 32K context (already generous for 12GB VRAM with 9B Q4 weights + KV), schemas consume the majority of the window, leaving scraps for history, Vein context, and the repo map. There is also a second-order cost: a 9B model selecting one tool out of 288 schemas is a far harder routing problem than selecting from 15 — schema broadcast actively degrades tool-call accuracy.
- **Fix:** routing-driven tool subsetting (§5.2). The `needs_*` predicates already compute the answer; use them to build `turn_tools` instead of (or in addition to) steering text. A core set of ~12 tools (file ops, grep, shell-or-not, inspect_host, run_code, verify) plus routed families should land most turns under 4K tokens of schema.

### 4.2 BLOCKING — CI does not run the test suite

- **Where:** `.github/workflows/ci.yml` (entire file — 43 lines: fmt, audit, deny; `on: workflow_dispatch | tags`).
- **Why it matters:** ~7,400 test functions exist and never gate anything. Every release tag is cut from code whose tests were last run on the author's machine, on Windows only. For a project whose pitch is *correctness via determinism*, this is the first thing a skeptical engineer checks and the fastest way to lose them. It also means routing regressions — explicitly a correctness property here — ship silently.
- **Fix:** push/PR-triggered matrix (windows-latest + ubuntu-latest): `cargo clippy --all-targets -- -D warnings`, `cargo test --lib --tests` (with platform-gated host_inspect tests), keep fmt/audit/deny. Cache with `Swatinem/rust-cache` (already used in releases). Time-box the math suite if needed via `--test` partitioning.

### 4.3 SERIOUS — `run_turn` monolith (~8,000 lines) and `conversation.rs` (15,041 lines)

- **Where:** `src/agent/conversation.rs:3082-~11130`.
- **Why it matters:** the harness's central correctness property — "what happens on a turn" — is a single function with dozens of mutable loop-local trackers (8677-8720), interleaved policy, steering, budgeting, dispatch, recovery, and rendering concerns. It cannot be unit-tested (no test constructs a `ConversationManager` and drives the loop with a fake provider), cannot be reviewed incrementally, and every fix risks adjacent breakage. This is also the file outside contributors must touch most.
- **Fix:** mechanical extraction first, behavior changes never in the same commit: (1) deterministic slash-command lane → `agent/deterministic_lane.rs`; (2) intervention chain → `agent/interventions.rs` with a declarative `Vec<Intervention>` table; (3) the iteration loop's tracker state → a `TurnState` struct with methods; (4) budget/pressure code already half-extracted to `economics.rs` — finish it. Then add the missing seam: a `Provider` fake so the loop can be integration-tested headlessly (§7.2).

### 4.4 SERIOUS — Routing precision is hand-ordered string matching with 0.4% test coverage

- **Where:** `src/agent/routing.rs:558-2813` (topic router), 5540+ (240 `needs_*` predicates); `tests/routing_precision.rs` (27 tests).
- **Why it matters:** the project treats routing as a correctness property (correctly), but encodes it as ~12K lines of ordered `contains` chains where *insertion order is the priority system*. Git history shows recurring "fix routing gaps" commits (`c7249ed`, `4b7cccb`) — each fix risks reordering collisions the 27 tests don't cover. There is no negative-case corpus ("queries that must NOT match topic X"), no fuzz/property layer, and everything is English-only.
- **Fix:** §5.2 / §7.6 — a data-driven routing table + a golden corpus of several hundred labeled queries (positive and negative), run in CI, with a coverage report per topic and per predicate.

### 4.5 SERIOUS — Python sandbox is bypassable; module claims otherwise

- **Where:** `src/tools/code_sandbox.rs:4` ("no filesystem escape"), 83-196 (`run_python`, `wrap_python`).
- **Why it matters:** the Scientific Mandate tells the model every number must come from `run_code` — so `run_code` output is treated as ground truth. A monkey-patched in-process "sandbox" is fine as an *accident guard* but it is documented as a security boundary, and the filesystem is fully open (deliberately — comment at 113 even preserves `tempfile`). Model-generated Python runs with the user's full privileges. One prompt-injected snippet in a README the agent is asked to "analyze and run" defeats it.
- **Fix:** (1) immediately: correct the claims in the module doc and tool description — call it "best-effort isolation; treat Python as trusted-operator execution"; (2) short-term: prefer Deno for everything it can do, route Python through `-I` plus an OS-level jobobject/cgroup wrapper where available; (3) document the residual trust assumption honestly so the Scientific Mandate's "ground truth" framing isn't built on a sandbox that isn't one.

> **Audit note (verified 2026-06-10):** I independently confirmed this. `wrap_python` (`code_sandbox.rs:143-196`) reassigns `_os.system`/`_os.popen` and overrides `builtins.__import__` with a hardcoded blocklist, but `_real_import` remains callable, `ctypes` is not blocked, and `open()`/file I/O is unrestricted. Restoring `builtins.__import__ = _real_import` or importing `ctypes` defeats it in one line. It is an accident guard, not a security boundary, and the line-4 "no filesystem escape" comment is inaccurate.

### 4.6 SERIOUS — Swarm fanout is not VRAM-safe under load

- **Where:** `src/agent/swarm.rs:79-171` (`dispatch_swarm`), wired into the TUI at `tui.rs:3469, 4241` and `conversation.rs:12352`.
- **Why it matters:** "swarm parallel agents on constrained VRAM" is a headline feature, but the safety mechanism is a *single* `gpu_state.ratio() > 0.85` snapshot taken once before the loop (`swarm.rs:89-90`). The moment fanout begins the snapshot is stale; there is no per-worker admission check, no per-worker VRAM estimate, and no model-size accounting. Excess tasks beyond `max_workers` are silently dropped (`.take(max_workers)`, line 104) and worker failures are swallowed (`if let Ok(res) = …`, line 121) — both violate the project's own "honest failure" principle. On a 12 GB card, loading a worker model alongside the main 9B + embeddings can OOM the GPU, and the current guard cannot prevent it.
- **Fix:** §5.7 — per-admission VRAM check before each `spawn`, concurrency capped at `min(max_workers, headroom / per_worker_estimate)`, sequential degradation when headroom is unknown, surfaced (not swallowed) worker errors, and an explicit operator message when tasks are dropped. Finish or delete the `#[allow(dead_code)]` `apply_patches_descending` and remove the non-professional comments (`swarm.rs:173, 233`).

### 4.7 MODERATE — MCP "serious enterprise surface" overstated; arg allowlist is global

- **Where:** `src/agent/mcp_server.rs:312-339` (`sanitize_args` — one flat `ALLOWED` key list shared by all tools), `mcp_server.rs:343-349` (schema/description claims).
- **Why it matters:** today only `inspect_host` is exposed and it is read-only, so the blast radius is small and the design is sound *for stdio* (the transport is the trust boundary). But the README/mission frame this as an enterprise integration surface, and as written there is no auth, no rate limiting, and a permissive arg allowlist that is not derived from each tool's declared schema. Adding any second tool silently shares the same key set; adding any *write* tool without first deriving per-tool ACLs would be dangerous. The tool description also claims "Works on Windows, Linux, and macOS" while many topics are Windows-only — overstated.
- **Fix:** §5.6 — derive the arg allowlist from the called tool's `inputSchema.properties`; add optional bearer-token auth + per-connection rate limiting (off by default for stdio, *required* before any TCP/SSE transport); keep read-only-by-default and gate any write tool behind explicit policy.

### 4.8 MODERATE — Conversation budget trimmer and preflight disagree by the entire tool schema

- **Where:** `enforce_prompt_budget` counts messages only (`conversation.rs:13169, 13211`); `preflight_chat_request` counts messages **+ tool schema** (`inference.rs:1102-1141`).
- **Why it matters:** the trimmer can successfully shrink history under its 0.68×ctx target and *still* have preflight hard-fail, because the ~31K-token tool array is invisible to the trimmer. The operator sees the budget ladder "succeed" and then the turn blocked — a confusing, seemingly contradictory failure. This is a direct consequence of §4.1 and disappears once tool subsetting lands, but until then the two accountings should at least agree.
- **Fix:** include the selected tool-schema token cost in `estimate_prompt_tokens` so the trimmer and preflight share one budget model.

### 4.9 MODERATE — 413 flat CLI flags, no subcommands, largely untested

- **Where:** `CliCockpit` (`src/lib.rs:113+`, 413 `#[arg]` fields); headless dispatch (`main.rs:106-200+`).
- **Why it matters:** the deterministic headless product is substantial and is the most reproducible value in the repo, but a single flat clap struct with 413 flags and no subcommands is hard to discover (`--help` is a wall), hard to validate (which flag combinations are legal?), and almost entirely untested — there is no smoke test asserting that each report format or each `--fix-all --only <label>` path produces valid output. Flags that only make sense together (`--fix --execute --yes`, `--diagnose --dry-run`) have no structural grouping.
- **Fix:** §7.4 — a headless smoke matrix covering each flag family and every `--report-format`; medium-term, consider clap subcommands (`hematite report`, `hematite diagnose`, `hematite mcp`, `hematite analyze`) to make the surface legible and the illegal combinations unrepresentable.

### 4.10 MODERATE — Windows assumptions leak into shared (non-`cfg`-gated) code

- **Where:** `prompt.rs:239` hardcodes `"- Operating System: Windows (User workspace)"` unconditionally; `prompt.rs:233-238` reads `USERPROFILE`/`COMPUTERNAME` with no Unix fallback; global guidance file lookup uses `USERPROFILE` only (`prompt.rs:137`).
- **Why it matters:** Linux + macOS are *release* targets (`unix-release.yml`), yet the system prompt tells the model the OS is Windows even on Linux, so a Linux user gets PowerShell suggestions. `host_inspect.rs` is well-`cfg`-gated (132 windows / 103 non-windows blocks); the prompt layer is not held to the same discipline.
- **Fix:** make the env block OS-aware via `std::env::consts::OS`; mirror the `cfg` discipline already present in the inspection layer; fall back to `HOME` when `USERPROFILE` is absent.

### 4.11 MODERATE — Conversation loop can strand a turn in an optimistic placeholder

- **Where:** `conversation.rs:8684` (`max_iters = 25`), empty-response nudges capped at 2 (`10217+`), deterministic closeout fallback emitting `[Proof successful. See tool output above…]` (`~10256`). Several intervention branches `continue` the loop without consuming a unified retry budget (e.g. `implement_current_plan` nudge `~10205`, scaffold auto-architect `8656-8675`).
- **Why it matters:** "honest operator-visible failure states" is a headline goal. Distinct recovery paths each carry their own retries that can compound toward the 25-iteration ceiling and end in an *optimistic* placeholder string when nothing was actually proven — the opposite of an honest failure state.
- **Fix:** §5.1 — a single per-turn `RetryBudget` consumed by every `continue` path; on exhaustion emit a typed `RuntimeFailureClass` checkpoint, never an optimistic placeholder.

### 4.12 MODERATE — Tool-output overflow handling is inconsistent across 270+ modules

- **Where:** `read_file`/`inspect_lines` get a compact-context 3,000-char cap with a nav hint (`conversation.rs:9690-9705`); `shell` is capped at 64 KB (`tool_registry.rs:30`). Most other tool modules return whatever `execute()` produces, with no central truncation/scratch-file policy.
- **Why it matters:** on a 9B's tiny context, one large `csv_tools`/`pcap_tools`/`elf_tools` result can blow the budget with no navigation affordance — exactly the failure the read-file cap was designed to prevent, but applied to only two tools.
- **Fix:** §5.5 — a uniform post-dispatch overflow policy in `dispatch_tool` (truncate + write full output to `.hematite/scratch/` + return a pointer), not per-tool ad hoc.

### 4.13 MINOR — `unsafe impl Send/Sync for Vein`, documentation drift, voice startup cost

- `vein.rs:52-53` — `unsafe impl Send/Sync` justified by an `Arc<Mutex<Connection>>` comment that is sound today but fragile: the embedding cache is a *separate* `RwLock`, so the invariant spans more than one field and is undertested. Add a concurrent-access stress test or move to a fully `Mutex`-guarded connection.
- Topic-count drift: `prompt.rs:119` "81+", `mcp_server.rs` "116+", docs "128+", actual resolvable arms 127. `verify-doc-sync.ps1` exists but isn't in CI.
- Voice eagerly constructs the ONNX session at TUI startup on a dedicated 32 MB-stack thread (`voice.rs:56-91`), adding cold-start latency and ~355 MB to the portable artifact. Mitigated by visible status messages; no smoke test asserts audio-within-N-seconds.

---

## 5. Recommended Target Architecture

Modular evolution from what exists — no rewrite. Every proposal is feasible on a 9B / 12 GB target.

### 5.1 Conversation loop hardening

- Extract a pure `TurnPlanner`: `(user_input, history, intent, budget) → TurnPlan { selected_tools, ordered_interventions, recovery_ladder }`. `run_turn` becomes a thin executor over the plan. This is the seam that makes the loop testable (§4.3).
- One unified per-turn `RetryBudget` (e.g. 3 transient + 2 empty-response + 1 verification re-ask) consumed by **all** `continue` paths. Exhaustion → typed `RuntimeFailureClass` checkpoint, never an optimistic placeholder (§4.11).
- Explicit recovery ladder, each rung observable in the SPECULAR panel: transient retry → narrowed re-ask (drop tools, restate goal) → grounded fallback (last tool result) → honest typed failure.

### 5.2 Routing architecture — eliminate shell fallback as a *structural* property

- Replace `turn_tools = self.tools.clone()` (`conversation.rs:8900`) with `select_tools(intent, input)`: a fixed core set (read/write/edit/grep/list/run_code/inspect_host/verify) plus the tool families whose `needs_*` predicates fired. Cap the per-turn tool payload at ≤ 15% of live context.
- When a `needs_*` predicate fires for a query that has a dedicated tool, **remove `shell`** from `turn_tools` for that turn (generalize the existing `sovereign_mode` filter at `8892-8898`). Routing becomes enforced by toolset shaping, not advisory prompt text.
- Promote the routing table from ~12K lines of hand-ordered `contains` chains to a data-driven table with positive *and* negative labeled cases (§7.6). A query with a native tool should be *structurally incapable* of emitting `shell`.

### 5.3 Context budget — make the 9B's context work harder

- **Tool-schema budgeting is the top priority** (§4.1). Two-stage exposure: always send compact tool *names + one-liners* (the machinery already exists in `build_system_prompt_compact`, `inference.rs:766-773`), expand full JSON schema only for the handful `select_tools` chose.
- Unify the budget model so `enforce_prompt_budget` (`conversation.rs:13211`) and `preflight_chat_request` (`inference.rs:1123`) count the same things (§4.8).
- Keep the five-stage message-trim ladder, but run it *after* tool selection so it compensates for conversation size, not schema bloat.

### 5.4 Vein RAG quality

- Add a precision/recall golden harness (`tests/vein_relevance.rs`): a fixture repo, labeled `query → expected-top-file` pairs, assert recall@5, and assert hybrid > BM25-only when embeddings are present.
- Emit an operator-visible "semantic tier OFF (no embed model loaded)" note so hybrid degradation is honest (today it silently no-ops, `vein.rs:599-601`).
- Prefer `chunk_by_symbols` for all tree-sitter-supported languages; reserve `sliding_window_chunks` for unknown types. Tune overlap against the golden set, not by hand.
- Add an exact-symbol-match rerank boost when the query contains an identifier present in a chunk.

### 5.5 Tool correctness guarantees

- A golden-output harness across **all** dispatchable tools: for each `match` arm in `dispatch_builtin_tool`, ≥1 `(input → expected output)` fixture, run in CI. This is the only way to prevent silent wrong-output regressions across 270+ modules.
- Property tests for reversible tools (encode/decode, base/radix, diff/patch): `decode(encode(x)) == x`.
- Uniform post-dispatch overflow → scratch-file policy (§4.12).

### 5.6 MCP server hardening

- Per-tool arg allowlist derived from `inputSchema.properties` (§4.7).
- Optional bearer-token auth + per-connection rate limit, off by default for stdio, required before any socket transport.
- Read-only by default; any write tool opt-in behind explicit policy. Correct the cross-platform claim in the tool description.

### 5.7 Swarm on constrained hardware

- Per-admission VRAM check; `concurrency = min(max_workers, headroom / per_worker_estimate)`; sequential when headroom is unknown.
- Account for worker model load/unload against the 12 GB ceiling using the existing `gpu_monitor` telemetry.
- Surface (don't swallow) worker errors and dropped-task notices. Finish or delete `apply_patches_descending`. Remove the non-professional comments.

### 5.8 Test architecture — see §7.

### 5.9 Cross-platform correctness

- OS-aware prompt env block (§4.10).
- CI test matrix on windows **and** ubuntu (§4.2). Linux must be a *tested* target, not just a release artifact.

### 5.10 Packaging and distribution

- Keep the portable zip + Windows installer.
- Ship a **voice-less** `crates.io` build (already feature-gated) so `cargo install hematite-cli` works without the ~355 MB weight bundle.
- Document the verified three-model 4070 stack as a one-command setup; consider `winget`/`scoop` manifests once CI is trustworthy.

---

## 6. Concrete Roadmap

### Stage 0 — Repo hardening (1–2 weeks)
- **Must-have:** CI `pull_request` + `push:main` trigger running `cargo clippy --all-targets -D warnings` + the full test matrix on windows + ubuntu (§4.2). Tool-subsetting MVP: `select_tools(intent)` replacing `self.tools.clone()` with a ≤15%-context cap (§4.1). Unify retry budget (§4.11). OS-aware prompt env block (§4.10). Correct the Python-sandbox doc claims (§4.5).
- **Tests:** existing suites must pass in CI; add "query must not select `shell` when a native tool exists."
- **Docs:** reconcile topic count to one source of truth; wire `verify-doc-sync.ps1` into CI.
- **Acceptance:** required green CI on every PR; a 16k-context coding request never trips `context_window_blocked`.
- **Risks reduced:** §4.1, §4.2, §4.5(claims), §4.10, §4.11, §4.13(drift).

### Stage 1 — Credible local-first MVP (2–4 weeks)
- **Must-have:** everything the README claims works on the verified 4070 stack. Extract `TurnPlanner` (§4.3). Uniform tool-output overflow policy (§4.12). Vein relevance honesty + golden test (§4.4 retrieval). Unify trimmer/preflight budget (§4.8).
- **Tests:** `TurnPlanner` unit sheet (mock provider); `tests/vein_relevance.rs`; headless-flag smoke tests for every report format (§4.9).
- **Docs:** a single "Verified on RTX 4070" setup page; begin trimming `CLAUDE.md`/`CAPABILITIES.md` toward accuracy.
- **Acceptance:** a cold-clone contributor reproduces the headline demo with no surprises.
- **Risks reduced:** §4.3, §4.8, §4.9, §4.12.

### Stage 2 — Harness maturity (4–8 weeks)
- **Must-have:** demonstrate the harness beats raw model access. Two-stage tool exposure (§5.3). Routing enforced via toolset shaping (§5.2). Recovery ladder fully observable.
- **Tests:** an automated A/B eval suite (`evals/` is human-scored today) comparing harness vs. raw provider on a fixed task set; routing regression at scale (positive + negative corpus, §7.6).
- **Docs:** an honest harness-vs-raw benchmark on a 4070.
- **Acceptance:** measurable task-success delta; no turn ends in an optimistic placeholder.
- **Risks reduced:** §4.4 (routing), residual §4.1/§4.11.

### Stage 3 — Serious platform (8–12 weeks)
- **Must-have:** MCP hardening (§5.6), VRAM-aware swarm (§5.7), tool-correctness golden harness (§5.5), voice smoke test, real Python isolation (§4.5).
- **Tests:** MCP protocol conformance; swarm OOM-safety simulation; full tool golden sheet in CI; voice audio-within-N-seconds.
- **Acceptance:** swarm cannot OOM a 12 GB card under load test; MCP passes conformance; every dispatchable tool has a golden.
- **Risks reduced:** §4.5(isolation), §4.6, §4.7, §4.13.

### Stage 4 — Community-ready (ongoing)
- **Must-have:** `run_turn` decomposed enough that an outsider can add a tool + routing + test without touching the monolith. `CONTRIBUTING.md` reflects the real gate; `justfile`'s "mirrors CI exactly" becomes true. Architecture docs match code.
- **Tests:** contribution template requires a test; CI blocks untested tools.
- **Acceptance:** an external contributor lands a new tool end-to-end via the documented path.
- **Risks reduced:** maintainability, contributor trust.

---

## 7. Testing and Verification Plan

Current state: a large corpus (~7,100 integration tests + 252 in-module) but **no CI gate**, uneven coverage (math + diagnostics heavy; loop + RAG + cross-platform light), and `just test` omits the math/scientific/data suites (justfile:26-30).

### 7.1 Unit — tool correctness
Golden `(input → output)` fixture per dispatchable tool; property tests for reversible tools (`decode∘encode == id`). `math_tools.rs` already does this for math — generalize to all 270+ modules and *run them in CI*.

### 7.2 Integration — conversation loop
The biggest current gap. Add a mock `Provider` (the `provider.rs` trait makes this clean) returning scripted tool-calls/empties/errors; construct a `ConversationManager` and drive `run_turn`; assert the recovery ladder, retry-budget exhaustion → typed failure, and that no turn emits an optimistic placeholder. This is impossible until §4.3 extraction creates the seam.

### 7.3 Integration — Vein retrieval
Fixture repo + `query → expected-top-file` recall@5 sheet; assert hybrid > BM25-only with embeddings present and an honest note when absent.

### 7.4 CLI / headless
Smoke every flag family (`--report/--diagnose/--triage/--fix/--fix-all/--inspect/--query` + the data-analysis batch) and every `--report-format` (md/json/html). 413 flags ≠ 413 tested behaviors today (§4.9).

### 7.5 MCP conformance
Drive `hematite --mcp-server` through `initialize/tools/list/tools/call`; assert JSON-RPC framing, error codes, redaction headers, policy blocks, and that semantic-tier failure returns an error, not raw data (`mcp_server.rs:281`).

### 7.6 Routing regression at scale
Promote `routing_precision.rs` (27 cases) to a data-driven sheet of several hundred labeled queries — positive (`query → expected_topics/tools`) **and** negative (`must_not_select: ["shell"]`) — with a per-topic / per-predicate coverage report, run in CI.

### 7.7 Privacy boundary
Assert raw username/MAC/hostname never appear in regex or semantic output; assert the semantic fail-safe.

### 7.8 Cross-platform CI
Windows-specific (`host_inspect` CIM paths) and Linux-specific (`/proc`, systemd) tests on their own runners.

### 7.9 Performance regression
Startup time, first-inference latency, Vein query time, **and per-turn tool-payload token count** (this last one guards §4.1 from silently regressing).

### 7.10 Voice smoke
Under `embedded-voice-assets`, assert synthesis yields non-empty audio within a latency budget.

---

## 8. Open Source Strategy

**Brutally honest.**

**What makes it credible / interesting:**
- *r/rust & HN:* lead with the **deterministic, model-free IT product**. A single Rust binary doing grounded host inspection across 127 topics, with three-tier local redaction and an MCP server, **no cloud, no API key**, is a real "Show HN." Do *not* lead with "AI coding agent" — that invites a head-to-head with Claude Code/Cursor on a battlefield a 9B can't win, and the README already concedes as much.
- *Burned sysadmins:* `hematite --diagnose` / `--fix-all` with zero model dependency and operator-visible failure states. "It tells you exactly what it can't see and why" is the pitch.
- *Skeptical AI engineers:* the honest-harness framing — "we don't pretend the 9B is smart; here's the scaffolding, here's the A/B eval vs. raw." That requires the Stage 2 eval data to actually exist.
- *Windows power users:* CRLF-safe edits, PowerShell-native, teleportation, baked-in voice — real terminal tooling, not a rough Linux port.

**The most impressive *reproducible* 4070 demo (no cloud, no keys):**
A side-by-side where a cloud agent calls Hematite's MCP `inspect_host` with `--semantic-redact`, and the operator watches a **local** Bonsai-8B summarize and scrub the output so username/hostname never leave the machine — verified clean in the final payload. The README already documents a verified ~805-token / ~82 s run; it runs entirely on a 4070; no cloud-only tool can tell this story. **That is the demo.** Pair it with `hematite --diagnose` producing a real health report with no model at all.

**What would make it look amateurish:**
- The 288-tools-every-turn budget bug (§4.1): a Rust engineer who reads `run_turn` sees a 9B fed ~31K tokens of tool schema and concludes "engineered for local constraints" is marketing. **Fix before any public post.**
- A Python "sandbox" documented as a security boundary that a one-liner defeats (§4.5).
- `swarm.rs` prose ("Cross the dimensional bound!", "organically slicing VRAM limits natively!") in shipped code.
- **No CI build/test/clippy gate** on a 400K-LOC repo (§4.2) — the first thing a skeptic checks.
- 383 KB `CLAUDE.md` and inconsistent topic counts — reads as AI-generated and unmaintained.

**What would make it impressive:**
- A tight, *subtractive* core: fewer tools, each golden-tested, routed precisely, with the tool budget visibly under control.
- Honest harness-vs-raw eval numbers on named hardware.
- Green CI on windows + linux, clippy-clean, `cargo install`-able (voice-less).

The credibility problem is not capability — it's *over-claiming breadth while under-proving correctness*. Narrow the public story to what is deterministic and reproducible, prove it in CI, and the Rust/sysadmin audience will take it seriously.

---

## 9. Top 20 Implementation Tasks

Prioritized: correctness → reliability → capability.

1. **Per-turn tool subsetting.** Goal: replace `self.tools.clone()` with `select_tools(intent, input)` capping tool payload ≤ 15% of live context. Files: `conversation.rs:8886`, new `agent/tool_select.rs`, `routing.rs`. Why: §4.1, the blocking budget bug. Tests: token-budget assertion per context tier; "core tools always present." Acceptance: 16k-context coding request never trips `context_window_blocked`.
2. **CI build/test/clippy on PRs.** Goal: `pull_request` + `push:main` triggers running clippy `-D warnings` + `just test` on windows + ubuntu. Files: `.github/workflows/ci.yml`. Why: §4.2. Tests: the workflow itself. Acceptance: PRs blocked on red.
3. **Correct the Python-sandbox security claims.** Goal: fix the line-4 "no filesystem escape" doc + tool description; reframe as best-effort isolation. Files: `tools/code_sandbox.rs:1-10`, `tool_registry.rs` run_code description. Why: §4.5 (cheap, high-integrity). Tests: a test asserting the description no longer claims a security boundary. Acceptance: docs match reality.
4. **Enforce shell-removal when a native tool is routed.** Goal: extend the `sovereign_mode` filter so any fired `needs_*` with a dedicated tool drops `shell` from `turn_tools`. Files: `conversation.rs:8886`, `routing.rs`. Why: §4.4/§5.2, routing as correctness. Tests: "query X must not select shell." Acceptance: docker/csv/http queries never yield `shell`.
5. **Unify per-turn retry budget.** Goal: single `RetryBudget` consumed by all `continue` paths; exhaustion → typed failure, not placeholder. Files: `conversation.rs:8684-10260`. Why: §4.11. Tests: mock-provider loop tests. Acceptance: no turn emits "Proof successful" without a tool result.
6. **OS-aware prompt environment block.** Goal: stop hardcoding "Operating System: Windows"; branch on `std::env::consts::OS`; `HOME` fallback. Files: `prompt.rs:137, 222-239`. Why: §4.10. Tests: prompt snapshot on linux. Acceptance: Linux runs never advertise Windows/PowerShell.
7. **Extract `TurnPlanner`.** Goal: pure planner (input→plan) + thin executor; mechanical, no behavior change. Files: `conversation.rs` → `agent/turn_planner.rs`. Why: §4.3. Tests: planner unit sheet. Acceptance: intervention ordering covered by unit tests.
8. **Mock-provider integration harness.** Goal: a `Provider` impl returning scripted responses to drive `run_turn`. Files: new `tests/loop_recovery.rs`, `agent/provider.rs`. Why: §4.3/§7.2 testability. Acceptance: recovery ladder covered.
9. **Two-stage tool exposure.** Goal: names+one-liners for all; full schema only for selected tools. Files: `tool_registry.rs`, `inference.rs:706-773`. Why: §5.3. Tests: payload-size test. Acceptance: full schemas ≤ selected count.
10. **Unify trimmer/preflight budget model.** Goal: include selected tool-schema cost in `estimate_prompt_tokens`. Files: `conversation.rs:13169`, `inference.rs:1102`. Why: §4.8. Tests: a case where trimmer succeeds ⇒ preflight passes. Acceptance: no "budget OK then blocked" contradiction.
11. **Vein relevance golden test + honesty.** Goal: recall@5 sheet; operator note when semantic tier off. Files: `vein.rs:599, 838`, `tests/vein_relevance.rs`. Why: §4.4/§7.3. Acceptance: retrieval regressions fail CI.
12. **Uniform tool-output overflow policy.** Goal: central truncate-to-scratch in `dispatch_tool`. Files: `conversation.rs:11138`, `tools/file_ops.rs`. Why: §4.12. Tests: oversized-output test. Acceptance: no single tool result exceeds the cap.
13. **VRAM-aware swarm admission.** Goal: per-spawn headroom check; concurrency capped by estimate; surfaced errors/dropped-task notices. Files: `swarm.rs:79-171`. Why: §4.6. Tests: simulated OOM guard. Acceptance: load test cannot OOM 12 GB.
14. **Reconcile topic count to one source of truth.** Goal: single constant feeding docs + MCP schema + prompt; wire `verify-doc-sync.ps1` into CI. Files: `host_inspect.rs`, `mcp_server.rs`, `prompt.rs:119`, docs. Why: §4.13. Acceptance: doc-sync check green in CI.
15. **MCP per-tool arg allowlist.** Goal: derive `ALLOWED` from `inputSchema.properties`. Files: `mcp_server.rs:312-339`. Why: §4.7. Tests: arg-injection test. Acceptance: unknown keys dropped per declared schema.
16. **Routing regression corpus.** Goal: data-driven positive+negative labeled queries, per-topic coverage. Files: `tests/routing_precision.rs` → data-driven sheet, `routing.rs`. Why: §4.4/§7.6. Acceptance: routing changes gated by the corpus in CI.
17. **Golden-output harness for all dispatchable tools.** Goal: ≥1 fixture per tool in CI. Files: `tests/tool_golden.rs`, `tool_registry.rs`. Why: §5.5. Acceptance: every `match` arm has a golden.
18. **Headless-flag smoke matrix.** Goal: test each report flag + every `--report-format`. Files: `tests/headless_cli.rs`, `main.rs`. Why: §4.9. Acceptance: all formats produce valid output.
19. **Real Python isolation.** Goal: `-I` + OS-level jobobject/cgroup wrapper; prefer Deno where possible. Files: `tools/code_sandbox.rs:83-196`. Why: §4.5 (after the doc fix in #3). Tests: escape-attempt tests fail safely. Acceptance: documented isolation matches behavior.
20. **`crates.io` voice-less install path + doc trim.** Goal: ensure `cargo install hematite-cli` builds without weights; split/trim `CLAUDE.md`/`CAPABILITIES.md` toward accuracy. Files: `Cargo.toml`, `voice.rs`, docs. Why: §5.10/§8. Acceptance: clean `cargo install` on a fresh machine; docs reviewed against code.

---

## 10. Final Verdict

**Is this repo on a credible path toward its stated mission?**
For the *deterministic sysadmin / inspection / headless-IT* half — yes, clearly. That part is real, grounded, reproducible on a 4070 with no cloud, and embodies "don't pretend the model is smarter than it is" exactly. For the *9B-as-coding-agent* half — not yet, because the conversational hot path broadcasts all 288 tool schemas every turn (~31K+ tokens by the project's own estimator), which contradicts the hardware constraint the project is built around, and it is validated by a CI pipeline that never runs its ~7,400 tests. Both are fixable without a rewrite, and the routing layer needed for the first fix already exists.

**The single most important thing to fix right now:**
Per-turn tool subsetting (§4.1 / Task 1). Stop sending the entire tool catalog every turn. Nothing else in the harness can work within a 9B's context budget until this is fixed, and it is a contained change because the `needs_*` routing machinery to select tools is already written.

**What a serious Rust engineer would say after reading the codebase:**
"Real scope and real grounded tooling — the inspection engine, the redaction pipeline, and the failure-honesty machinery are genuinely good. But `run_turn` is an ~8,000-line monolith, the model is fed the entire tool catalog every turn, there's no CI test gate on 400K lines, the Python 'sandbox' isn't one, and `swarm.rs` reads like a hackathon. The bones are strong; it's over-scoped and under-verified. Cut the surface, gate it in CI, prove the harness beats raw, and I'd actually use it."

**What to build in the next two weeks:**
1. Tool subsetting with a token-budget cap (Task 1).
2. CI: clippy + full test matrix on windows + ubuntu, gating PRs (Task 2).
3. Correct the Python-sandbox security claims (Task 3) — one hour, pure integrity.
4. Enforce shell-removal when a native tool is routed (Task 4).
5. Unify the retry budget so no turn ends in an optimistic placeholder (Task 5).

Those five turn the strongest, most defensible version of Hematite — grounded, honest, budget-aware, and CI-verified — from aspiration into something a skeptic can clone and trust.
