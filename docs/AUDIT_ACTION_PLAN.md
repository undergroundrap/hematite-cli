# Hematite — Audit Action Plan

**Source:** `docs/ARCHITECTURE_AUDIT_CLAUDE.md` (2026-06-10, v0.14.1)
**Status key:** `[ ]` pending · `[~]` in progress · `[x]` done

> **Swarm note (from maintainer):** Swarm is intentionally architected for multi-GPU /
> high-VRAM users (RTX 4090+, dual-card setups). The single-GPU 4070 path already falls back
> to sequential execution above 85% VRAM. The swarm work items below are about *code quality
> and honesty* (remove dead code, surface swallowed errors, professional comments) — not about
> questioning the multi-GPU architecture.

---

## Stage 0 — Repo hardening (do before any public post)

These are the items a skeptical engineer finds in the first 15 minutes. Every one is
either blocking-correctness or pure integrity work.

### 0-A  Per-turn tool subsetting  ⚠ BLOCKING
**Audit ref:** §4.1 / Task 1  
**Problem:** `turn_tools = self.tools.clone()` at `conversation.rs:8900` sends all 288 tool
schemas (~31K+ tokens by the project's own bytes/4 estimator) every turn. At the default
8,192-token context this exceeds the window before a single message is added. The `needs_*`
predicates in `routing.rs:5540+` already know which tool families are relevant — they are
used only to inject steering text, never to filter the schema.

- [ ] Create `src/agent/tool_select.rs` with `select_tools(intent: &QueryIntent, input: &str, context_limit: usize) -> Vec<Tool>`
  - Always include a fixed core set: `read_file`, `write_file`, `edit_file`, `list_files`, `grep_files`, `shell` (only when no native tool fires), `inspect_host`, `run_code`, `verify_build`
  - Layer in tool families whose `needs_*` predicates fired (e.g. `needs_csv_tools` → csv_tools, `needs_docker_ops` → docker tools)
  - Cap total payload at ≤15% of `context_limit`
- [ ] Replace `turn_tools = self.tools.clone()` at `conversation.rs:8900` with `select_tools(...)`
- [ ] Keep the yolo/sovereign filter intact; compose it with the new selection
- [ ] Add a token-budget assertion test: for every context tier (8k/16k/32k/128k), `select_tools` output fits in the cap
- [ ] Verify: a 16k-context coding request never trips `context_window_blocked`

### 0-B  CI build/test/clippy gate  ⚠ BLOCKING
**Audit ref:** §4.2 / Task 2  
**Problem:** `.github/workflows/ci.yml` triggers only on tags and manual dispatch; runs only
`cargo fmt --check`, `cargo audit`, `cargo deny`. No build, no clippy, no tests, no Windows
runner. ~7,400 tests have never gated a merge.

- [ ] Add `on: pull_request` and `on: push: branches: [main]` triggers to `ci.yml`
- [ ] Add job matrix: `windows-latest` + `ubuntu-latest`
- [ ] Add `cargo clippy --all-targets -- -D warnings`
- [ ] Add `cargo test --lib --tests` (gate platform-specific host_inspect tests with `#[cfg(...)]` already in place)
- [ ] Add `Swatinem/rust-cache` (already used in `windows-release.yml`)
- [ ] Time-box the math suite if total time exceeds 10 min: run `--test diagnostics --test routing_precision` on every PR; math suite on merge to main
- [ ] Wire `scripts/verify-doc-sync.ps1` into the CI matrix (Windows runner only)
- [ ] Verify: PRs blocked on red; `justfile`'s "mirrors CI exactly" becomes true

### 0-C  Correct Python sandbox claims  (1 hour, pure integrity)
**Audit ref:** §4.5 / Task 3  
**Problem:** `src/tools/code_sandbox.rs:4` says "no filesystem escape." `wrap_python` at
lines 143-196 monkey-patches builtins but `_real_import` remains callable, `ctypes` is not
blocked, and `open()` is unrestricted. The line-4 comment is inaccurate.

- [ ] Update the module-level doc comment in `code_sandbox.rs` lines 1-10: replace "no filesystem escape" with "best-effort accident guard; treat Python execution as trusted-operator access with full filesystem"
- [ ] Update the `run_code` tool description in `tool_registry.rs` to match: "Python: best-effort isolation (open file I/O permitted); Deno: scoped sandbox (--deny-net etc.)"
- [ ] Add a test asserting the tool description does NOT contain the phrase "no filesystem escape" or "sandbox" as a security claim (protects against regression)

### 0-D  Enforce shell removal when a native tool is routed
**Audit ref:** §4.4/§5.2 / Task 4  
**Problem:** the existing sovereign-mode filter at `conversation.rs:8892-8898` removes only
two tools. When a `needs_*` predicate fires for a dedicated native tool, `shell` is still
present in `turn_tools` — routing is advisory text, not structural.

- [ ] Extend the tool-selection logic in `select_tools()` (0-A): when any of the following
  predicates fire, exclude `shell` from the returned set:
  `needs_csv_tools`, `needs_docker_ops`, `needs_http_request`, `needs_json_tools`,
  `needs_yaml_tools`, `needs_git_ops` (and any other `needs_*` that maps to a dedicated tool)
- [ ] Add routing tests: "docker query must not include shell in turn_tools", "csv query must not include shell in turn_tools"
- [ ] Acceptance: `docker ps`, `git log`, `parse this CSV` never yield a `shell` call

### 0-E  Unified per-turn retry budget
**Audit ref:** §4.11 / Task 5  
**Problem:** `max_iters = 25` at `conversation.rs:8684`, but distinct recovery paths
(`implement_current_plan` nudge ~10205, scaffold auto-architect 8656-8675, empty-response
nudges 10217) each carry independent retries that can compound toward the ceiling and end
in an optimistic placeholder (`[Proof successful. See tool output above…]` at ~10256).

- [ ] Define a `TurnBudget` struct (or extend `economics.rs` if it already has one) tracking:
  - `transient_retries_remaining: u8` (e.g. 3)
  - `empty_response_retries_remaining: u8` (e.g. 2)
  - `verification_reasks_remaining: u8` (e.g. 1)
- [ ] Thread `TurnBudget` through the iteration loop; all `continue` paths consume from it
- [ ] On exhaustion: emit a typed `RuntimeFailureClass` checkpoint message; never emit the optimistic placeholder string unless a tool result is present
- [ ] Add mock-provider test (prerequisite: Task 8 mock harness): budget exhaustion → checkpoint, not placeholder

### 0-F  OS-aware prompt environment block
**Audit ref:** §4.10 / Task 6  
**Problem:** `prompt.rs:239` hardcodes `"- Operating System: Windows (User workspace)"` for
all platforms. `prompt.rs:137` uses `USERPROFILE` only (no Unix fallback). Linux/macOS builds
get PowerShell suggestions because the model is told it is on Windows.

- [ ] In `prompt.rs`, replace the hardcoded OS string at line 239 with:
  ```rust
  format!("- Operating System: {} (User workspace)", std::env::consts::OS)
  ```
  (capitalize for display if desired)
- [ ] Replace `USERPROFILE` lookups at lines 137 and 233-238 with:
  `std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))`
- [ ] Replace `COMPUTERNAME` at line ~235 with:
  `std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).or_else(|_| hostname::get()...)`
  (or skip on non-Windows)
- [ ] Add a test asserting the system prompt on Linux does not contain "Windows" or "PowerShell"

---

## Stage 1 — Credible local-first MVP

Everything the README claims works on the verified 4070 stack; foundation for outside contributors.

### 1-A  Extract TurnPlanner (mechanical, no behavior change)
**Audit ref:** §4.3 / Task 7  
**Problem:** `run_turn` at `conversation.rs:3082-~11130` is ~8,000 lines. It is effectively
untestable as a unit and unreviewable incrementally.

- [ ] Create `src/agent/turn_planner.rs`; extract the pure-function planning part:
  `plan_turn(input, history, intent, budget) -> TurnPlan { selected_tools, ordered_interventions, initial_recovery_ladder }`
- [ ] Create `TurnState` struct for the loop-local mutable trackers (8677-8720): iteration count, consecutive errors, repeat-call maps, read/grep dedup sets, mutation tracker
- [ ] Extract the deterministic slash-command lane (3091-3225) into `src/agent/deterministic_lane.rs`
- [ ] `run_turn` becomes a thin executor: `plan = plan_turn(...); execute_plan(plan, &mut state)`
- [ ] Keep behavior identical: no routing changes, no tool changes, one commit per extraction
- [ ] Verify: `cargo test` passes unchanged after each extraction step

### 1-B  Mock-provider integration harness
**Audit ref:** §4.3/§7.2 / Task 8  
**Problem:** there is no way to drive `run_turn` without a live LM Studio instance, so
recovery paths, budget exhaustion, and retry logic cannot be tested in CI.

- [ ] Implement `struct ScriptedProvider` in `tests/` (uses the `provider.rs` trait) that
  returns a pre-programmed sequence of `InferenceResult`s (tool calls, empty responses, errors)
- [ ] Create `tests/loop_recovery.rs`:
  - Test: empty response × 2 → typed failure, not optimistic placeholder
  - Test: tool repeat × 3 → hard-stop intervention
  - Test: 25-iter ceiling → `RuntimeFailureClass::LoopCeiling`
  - Test: budget exhaustion (0-E) → checkpoint emitted
- [ ] Prerequisite: TurnPlanner extraction (1-A) provides the seam

### 1-C  Two-stage tool exposure
**Audit ref:** §5.3 / Task 9  
**Problem:** even after subsetting (0-A), sending full JSON schema for every selected tool
is wasteful. Tool names + one-liners exist already (`build_system_prompt_compact`,
`inference.rs:766-773`) but are not used in the tool-call path.

- [ ] In `tool_select.rs`, produce two payloads:
  - `compact_tools`: all tools as `{name, description_one_liner}` (no `inputSchema`)
  - `selected_tools`: full schema only for the handful `select_tools` chose
- [ ] On first iteration of `run_turn`, send `selected_tools` full schema + `compact_tools` as a context note
- [ ] Add a payload-size test: full schemas present only for selected count; remainder are one-liners

### 1-D  Unify trimmer/preflight budget model
**Audit ref:** §4.8 / Task 10  
**Problem:** `enforce_prompt_budget` at `conversation.rs:13169` counts only messages;
`preflight_chat_request` at `inference.rs:1102-1141` counts messages + tool schema. The two
disagree by the size of the entire tool array, so trimming can succeed and preflight can
still block — a confusing contradiction the operator sees as "budget OK, then error."

- [ ] Move `estimate_serialized_tokens` to a shared location (`economics.rs` or `inference.rs`)
- [ ] In `enforce_prompt_budget`, include the selected tool-schema cost via `estimate_serialized_tokens(&turn_tools)`
- [ ] Add a test: in a scenario where history fills 70% of context, trimming succeeds → preflight passes (no more budget contradiction)
- [ ] Note: this is downstream of 0-A; once tool subsetting is in, schema cost is small and this becomes a one-liner fix

### 1-E  Vein retrieval honesty + golden tests
**Audit ref:** §4.4/§7.3 / Task 11  
**Problem:** `vein.rs:599-601` silently no-ops when no embedding model is loaded (semantic
tier goes dark with no operator notification). Retrieval quality is asserted but never
measured.

- [ ] Add an operator-visible message (TUI `System` event) when the semantic tier degrades:
  "Vein: semantic search OFF — no embedding model loaded. BM25-only retrieval active."
- [ ] Create `tests/vein_relevance.rs`:
  - Write a small fixture repo into a temp dir
  - Assert recall@5: `search_context("what renders on startup")` returns the banner file in top 5
  - Assert hybrid > BM25-only when an embedding fixture is present (mock embedding endpoint)
- [ ] Retrieval-regression test failures must gate CI

### 1-F  Uniform tool-output overflow policy
**Audit ref:** §4.12 / Task 12  
**Problem:** `read_file`/`inspect_lines` have a compact-context 3,000-char cap with a nav
hint; `shell` is capped at 64 KB. Most of the 270+ other tools return unbounded output.
One large `pcap_tools` or `elf_tools` result can silently consume the entire context budget.

- [ ] In `dispatch_tool` at `conversation.rs:11138`, add a post-dispatch overflow check:
  if `result.len() > TOOL_OUTPUT_CAP` (suggest 8,192 chars):
  - write full output to `.hematite/scratch/<tool>_<timestamp>.txt`
  - return truncated result + "Full output saved to: <path>. Use read_file to retrieve it."
- [ ] `TOOL_OUTPUT_CAP` should be configurable (constant in `economics.rs`)
- [ ] `read_file` and `shell` already have caps — leave them; the new check is the backstop for the rest
- [ ] Add a test: oversized tool output is truncated + scratch file written

### 1-G  Topic count single source of truth
**Audit ref:** §4.13 / Task 14  
**Problem:** `prompt.rs:119` says "81+"; `mcp_server.rs:7,349` say "116+"; actual topic
match arms = 127; docs say "128+". Four different numbers, all wrong.

- [ ] Add a compile-time or test-time constant: `pub const INSPECT_HOST_TOPIC_COUNT: usize = N;`
  (derive `N` from the actual arm count in `host_inspect.rs`, or count via `verify-doc-sync.ps1`)
- [ ] Replace all hardcoded "81+", "116+", "128+" strings in `prompt.rs`, `mcp_server.rs`,
  `README.md`, `CLAUDE.md`, `CAPABILITIES.md` with the single authoritative number
- [ ] Wire `scripts/verify-doc-sync.ps1` into CI (Windows runner): fails if docs drift from source
- [ ] Acceptance: one number, consistent everywhere, CI-verified

### 1-H  Headless-flag smoke matrix
**Audit ref:** §4.9 / Task 18  
**Problem:** `CliCockpit` has 413 flags and no subcommands. The deterministic headless
product has no smoke tests asserting each report format or flag combination produces valid
output.

- [ ] Create `tests/headless_cli.rs` (or extend `tests/diagnostics.rs`)
- [ ] Cover each primary flag family:
  - `--report`, `--diagnose`, `--triage`, `--fix "slow"`, `--fix-all --dry-run`
  - `--inspect summary`, `--query "why is my PC slow"`, `--watch resource_load --count 1`
  - Each with `--report-format md`, `--report-format json`, `--report-format html`
- [ ] For JSON format: assert output parses as valid JSON with expected top-level keys
- [ ] For HTML format: assert output contains `<!DOCTYPE html>`
- [ ] Run on Windows runner only (many inspect topics are Windows-specific)
- [ ] Acceptance: all flag/format combos produce structurally valid output

---

## Stage 2 — Harness maturity

Demonstrate the harness actually beats raw model access.

### 2-A  Routing regression corpus
**Audit ref:** §4.4/§7.6 / Task 16  
**Problem:** `tests/routing_precision.rs` has 27 tests against a ~12K-line hand-ordered
`contains` chain. Every "fix routing gaps" commit risks silent regressions. No negative-case
corpus ("must NOT select X") exists.

- [ ] Expand `tests/routing_precision.rs` to a data-driven sheet (use a CSV or inline const array):
  - Positive cases: `(query, expected_topic_or_tool)` — at least 5 per topic family
  - Negative cases: `(query, must_not_select)` — especially `must_not_select: "shell"` for
    all queries that have a dedicated native tool
  - Target: 300+ labeled pairs covering all 127 inspect topics + all 240 `needs_*` predicates
- [ ] Add per-topic coverage report (any topic with < 3 test cases triggers a warning)
- [ ] Run the corpus in CI; routing changes must not reduce coverage
- [ ] Track the "fix routing gaps" commit pattern: if the same topic needs 3+ fixes, add a fuzz
  property test for that topic's keyword set

### 2-B  Automated eval vs raw provider
**Audit ref:** §8 (open source strategy) / Stage 2  
**Problem:** `evals/` is human-scored today. The README's "42% reduction in context-melt
regressions" claim has no reproducible benchmark in-repo.

- [ ] Design a fixed task set (suggest 20–30 tasks) covering: file edit, grep-and-report,
  host inspection query, math computation, refactor
- [ ] Run each task against (a) Hematite harness, (b) raw provider (no tool steering, no
  subsetting) on the same model/hardware
- [ ] Record: task success (binary), turns to completion, context tokens consumed
- [ ] Store results in `evals/results/DATE_MODEL.json`; add a summary to `evals/README.md`
- [ ] Acceptance: harness measurably outperforms raw on the fixed set; the benchmark is
  reproducible from a clean clone

---

## Stage 3 — Serious platform

### 3-A  Swarm cleanup (code quality + honesty, not architecture change)
**Audit ref:** §4.6 / Task 13  
**Maintainer note:** swarm is intentionally for multi-GPU users. The 85% fallback to
sequential is the right single-GPU safety. The issues below are code quality and honest
failure — not an architecture question.

- [ ] **Remove unprofessional comments** in `swarm.rs`:
  - Line 173: remove "Cross the dimensional bound!" (or replace with a technical comment)
  - Line 233: remove "organically slicing VRAM limits natively!" (or replace with description)
- [ ] **Surface swallowed worker errors**: replace `if let Ok(res) = result` at line 121 with
  explicit match; on `Err(e)`, emit a TUI `System` event: "Worker N failed: {e}"
- [ ] **Notify when tasks are silently dropped**: after `.take(max_workers)`, if
  `tasks.len() > max_workers`, emit a visible operator notice: "Swarm: {N} tasks exceed
  worker limit and were dropped. Run with more VRAM or split the task."
- [ ] **Remove or implement `apply_patches_descending`**: it has `#[allow(dead_code)]` at
  line 174. Either delete it or complete it and wire it in. Dead code with a suppress
  attribute signals unfinished work.
- [ ] **Per-admission headroom check**: before each `tokio::spawn`, re-read
  `gpu_state.ratio()` — the single pre-loop snapshot becomes stale the moment workers start.
  If headroom drops below threshold mid-dispatch, stop spawning (sequential remainder).
- [ ] Add a `#[test]` in `swarm.rs` simulating the headroom check with a mock GPU state

### 3-B  MCP per-tool arg allowlist
**Audit ref:** §4.7 / Task 15  
**Problem:** `sanitize_args` at `mcp_server.rs:312-339` uses one flat `ALLOWED` key set
shared by all tools. Adding a second tool would silently share the same key set.

- [ ] Change `sanitize_args` signature to accept the called tool's `inputSchema`:
  `fn sanitize_args(args: &Value, schema: &Value) -> Value`
- [ ] Build the allowed-key set from `schema["properties"].as_object().keys()`
- [ ] In `dispatch_tool_call`, pass the matching tool's schema to `sanitize_args`
- [ ] Add a test: an extra arg not in `inputSchema.properties` is stripped from the sanitized output
- [ ] Fix the cross-platform claim: update the tool description to accurately state which topics
  are Windows-only (or note "Windows: full coverage; Linux/macOS: partial")

### 3-C  Golden-output harness for all dispatchable tools
**Audit ref:** §5.5 / Task 17  
**Problem:** 270+ tool modules, flat dispatch at `tool_registry.rs:7954`, only math_tools.rs
has systematic golden coverage. Silent wrong-output regressions can ship undetected.

- [ ] Create `tests/tool_golden.rs` (or expand existing tool tests)
- [ ] For every `match` arm in `dispatch_builtin_tool`, add ≥1 `(input → expected output)` fixture:
  - The fixture should be deterministic (no timestamps, no live system calls, use mock args)
  - Priority order: tools that touch files, tools that parse binary formats, math tools (existing)
- [ ] Add property tests for reversible tools: `decode(encode(x)) == x` for
  base64, hex, url-encode, caesar, diff/patch, pack/unpack, etc.
- [ ] Run the full golden harness in CI on every PR
- [ ] Acceptance: every `match` arm has a passing golden; any arm that cannot be tested in
  isolation gets a `// TEST-EXEMPT: requires live system` comment documenting why

### 3-D  Real Python isolation
**Audit ref:** §4.5 / Task 19 (after 0-C doc fix)  
**Problem:** the Python sandbox is best-effort monkey-patching. The doc fix (0-C) is the
immediate integrity repair; this is the deeper fix.

- [ ] On Windows: wrap the Python subprocess in a Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
  to ensure cleanup even if the process escapes monitoring
- [ ] Pass `-I` (isolated mode) to the Python interpreter to disable user site-packages and
  `PYTHON*` env vars
- [ ] Extend the blocklist to include `ctypes`, `importlib`, `multiprocessing`
- [ ] Add escape-attempt tests: `__import__('ctypes')`, `builtins.__import__ = builtins.__import__.__class__.__init__._real_import`  
  → assert they are blocked or produce an error (not silently succeed)
- [ ] Prefer Deno routing: if a `run_code` request is JavaScript/TypeScript, always use Deno
  (real sandboxing); Python path for Python-only needs
- [ ] Note: the filesystem is intentionally open for Python (documented after 0-C); the goal
  here is subprocess-escape prevention and accidental network access, not full hermetic isolation

---

## Stage 4 — Community-ready (ongoing)

### 4-A  crates.io voice-less install + doc trim
**Audit ref:** §5.10 / Task 20  

- [ ] Verify `cargo install hematite-cli` (without `--features embedded-voice-assets`) builds
  clean on a fresh Linux machine and a fresh Windows machine
- [ ] Verify voice-disabled path shows a clear "Voice: not available in this build" message
- [ ] Begin a doc-trim pass on `CLAUDE.md` (383 KB) and `CAPABILITIES.md` (198 KB):
  - Remove tool descriptions that duplicate the tool's own source-level doc
  - Consolidate the shipped-feature roadmap items into a single "What shipped" section
  - Keep the behavioral guidelines and architecture sections; they are signal, not noise
- [ ] `README.md` (146 KB): ensure the top 3 screens are the deterministic sysadmin pitch,
  not the agent pitch (per §8 open source strategy)
- [ ] Add `winget` and/or `scoop` manifest stubs (can be empty/placeholder) once CI is green

### 4-B  run_turn decomposition (ongoing from 1-A)
**Audit ref:** §4.3  
**Goal:** an outside contributor can add a tool + routing + test without touching the
~8,000-line `run_turn`.

- [ ] After 1-A TurnPlanner is stable: extract the intervention chain
  (currently `if loop_intervention.is_none() && needs_X(...)` ladder at ~5084+) into
  `src/agent/interventions.rs` as a `Vec<Intervention>` table
- [ ] The `Intervention` type: `{ predicate: fn(&str, &QueryIntent) -> bool, inject: fn(...) -> SystemMessage }`
- [ ] Adding a new intervention = one new `Intervention` entry, not editing the monolith
- [ ] `CONTRIBUTING.md`: document the "new tool checklist" (new tool module → register in
  tool_registry.rs → add `needs_*` predicate → add routing test → add golden test)
- [ ] Acceptance: a new contributor follows the checklist and opens a working PR without
  touching `conversation.rs`

---

## Cross-cutting: `unsafe impl Send/Sync for Vein`
**Audit ref:** §4.13  
**Problem:** `vein.rs:52-53` — the invariant spans the `Arc<Mutex<Connection>>` + a separate
`RwLock`-guarded embedding cache. Sound today but fragile to future field additions.

- [ ] Add a concurrent-access stress test: spawn 8 threads, each calling `search_context` and
  `index_file` simultaneously; assert no panic or data race (Miri or loom if available)
- [ ] Document the invariant inline: which fields are protected by which lock, and what
  the `unsafe` justification covers
- [ ] Medium-term: consider a `Mutex<VeinInner>` wrapper to make the invariant structural

---

## Summary table

| Stage | Item | Audit ref | Effort | Acceptance |
|-------|------|-----------|--------|------------|
| 0-A | Per-turn tool subsetting | §4.1 / T1 | M | 16k request never blocked |
| 0-B | CI build/test/clippy | §4.2 / T2 | S | PRs gated |
| 0-C | Python sandbox doc fix | §4.5 / T3 | XS | Docs match reality |
| 0-D | Shell removal when native tool fires | §4.4 / T4 | S | docker/csv never shell |
| 0-E | Unified retry budget | §4.11 / T5 | M | No optimistic placeholder |
| 0-F | OS-aware prompt | §4.10 / T6 | XS | Linux prompt says Linux |
| 1-A | Extract TurnPlanner | §4.3 / T7 | L | Tests unchanged |
| 1-B | Mock provider harness | §4.3 / T8 | M | Recovery ladder covered |
| 1-C | Two-stage tool exposure | §5.3 / T9 | M | Full schemas only selected |
| 1-D | Unify trimmer/preflight | §4.8 / T10 | S | No budget contradiction |
| 1-E | Vein honesty + golden | §4.4 / T11 | M | Retrieval failures gate CI |
| 1-F | Uniform overflow policy | §4.12 / T12 | S | No unbounded tool output |
| 1-G | Topic count source of truth | §4.13 / T14 | XS | One number, CI-verified |
| 1-H | Headless smoke matrix | §4.9 / T18 | M | All formats structurally valid |
| 2-A | Routing regression corpus | §4.4 / T16 | L | 300+ labeled pairs in CI |
| 2-B | Automated harness eval | §8 / Stage 2 | L | Reproducible benchmark |
| 3-A | Swarm cleanup | §4.6 / T13 | S | Errors surfaced, dead code gone |
| 3-B | MCP per-tool arg allowlist | §4.7 / T15 | S | Unknown keys stripped |
| 3-C | Golden-output harness | §5.5 / T17 | XL | Every dispatch arm covered |
| 3-D | Real Python isolation | §4.5 / T19 | L | Escape-attempt tests pass |
| 4-A | crates.io + doc trim | §5.10 / T20 | M | Clean `cargo install` |
| 4-B | run_turn decomposition | §4.3 | XL | New tool via checklist, no monolith |
| — | unsafe Send/Sync stress test | §4.13 | S | Concurrent access safe |

**Effort key:** XS < 1h · S = 1-4h · M = half-day · L = 1-2 days · XL = 3+ days
