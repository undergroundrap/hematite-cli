# Hematite Maintainer Workflows

Concise reference for agents and contributors. Command-first. No prose.

**Non-negotiable rule: always test before committing. Never commit to fix a problem you haven't verified is actually fixed.**

---

## 1. Release Workflow

The standard path for every version bump. Run from a clean working tree.

```powershell
# Step 1 — bump version (updates Cargo.toml, README, CLAUDE.md, installer)
powershell -ExecutionPolicy Bypass -File bump-version.ps1 -Version X.Y.Z

# Step 2 — rebuild to regenerate Cargo.lock
cargo build

# Step 3 — verify all version surfaces are in sync
powershell -ExecutionPolicy Bypass -File scripts/verify-version-sync.ps1 -Version X.Y.Z -RequireCargoLock

# Step 4 — commit exactly these five files (never git add .)
git add Cargo.toml Cargo.lock README.md CLAUDE.md installer/hematite.iss
git commit -m "chore: bump version to X.Y.Z"

# Step 5 — tag and push (triggers CI)
git tag -a vX.Y.Z -m "Release vX.Y.Z"
git push origin main
git push origin vX.Y.Z

# Step 6 — wait for CI green on BOTH windows-release AND unix-release workflows
# Step 7 — only publish to crates.io after CI is green on all platforms
cargo publish -p hematite-cli
```

**When to bump:**
- `PATCH` — bug fixes, doc updates, routing fixes, test additions
- `MINOR` — new user-visible features, new inspect_host topics, new TUI commands
- `MAJOR` — breaking config/API changes or first stable release

**Do not bump** to test whether a fix works. Build the local portable first, verify live, then bump.

---

## 2. Pre-Release Verification Loop

Run before every release. All must pass before bumping.

```powershell
cargo fmt
cargo check --tests
cargo test --test diagnostics
cargo test --test routing_precision
powershell -ExecutionPolicy Bypass -File scripts/verify-doc-sync.ps1
```

For a targeted single test:
```powershell
cargo test --test diagnostics test_name_here -- --exact
```

---

## 3. Local Portable Build (Smoke Test)

Build the actual portable binary and update PATH before live-testing. Do this before bumping a version.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/package-windows.ps1 -AddToPath
```

Restart the terminal after running. The `hematite` command on PATH now points to the new build. Live-test the behavior before committing or bumping anything.

---

## 4. Routing Fix Workflow

Use when a query routes to `shell` instead of `inspect_host`, or routes to the wrong topic.

1. Check `preferred_host_inspection_topic()` in `src/agent/routing.rs` — if the topic has no matching `asks_*` variable, `host_inspection_mode` is never injected and the model free-forms.
2. Add the missing `asks_*` variable with natural-language phrases covering the query shape.
3. Add it to the dispatch chain (`if asks_X { Some("topic") }`). Order matters — more specific topics before generic ones (e.g. `asks_ports` before `asks_processes`).
4. Add the same phrase to the matching detector in `all_host_inspection_topics()` (multi-topic pre-run table — separate from single-topic routing).
5. Update the HOST INSPECTION MODE bullet list in `src/agent/conversation.rs` so the model knows to use the topic.
6. Add a `test_routing_detects_*` test in `tests/diagnostics.rs` covering 2–3 representative phrases.
7. Run `cargo test --test diagnostics`, build portable, verify the live query. Commit only after live test passes.

**Key distinction:** `preferred_host_inspection_topic()` controls single-topic routing. `all_host_inspection_topics()` controls the multi-topic harness pre-run (fires when 2+ topics detected). A topic can be in one and not the other — always check both.

---

## 5. Adding a New inspect_host Topic

When adding a new `topic` to `src/tools/host_inspect.rs`:

1. Implement the inspector function and wire it into the `match` block.
2. Add routing phrases to `preferred_host_inspection_topic()` in `src/agent/routing.rs`.
3. Add a detector to `all_host_inspection_topics()` in `src/agent/routing.rs`.
4. Add a bullet to the HOST INSPECTION MODE list in `src/agent/conversation.rs`.
5. Add a bullet to the capability list in `src/agent/prompt.rs`.
6. Update `CAPABILITIES.md` topic count and add a row to the matrix.
7. Update the topic count reference in `README.md` and `CLAUDE.md`.
8. Run `powershell -ExecutionPolicy Bypass -File scripts/verify-doc-sync.ps1` — must report SUCCESS.
9. Add tests in `tests/diagnostics.rs`: at minimum a header test and a routing detection test.
10. Build portable, verify live, then commit.

**Cross-platform rule:** if a parameter is Windows-only, silence it on Unix with `let _ = param;` — not `#[cfg]` removal, which causes missed warnings on the other platform.

---

## 6. Doc Sync Verification

The `verify-doc-sync.ps1` script checks that topic counts are consistent across all docs.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify-doc-sync.ps1
```

Expected output:
```
SUCCESS: All documentation is synchronized and grounded.
```

If it fails, the topic count in one of `README.md`, `CAPABILITIES.md`, or `CLAUDE.md` is out of sync with the actual count in `src/tools/host_inspect.rs`. Update the failing doc to match.

---

## 7. Crates.io Publish Rules

- Publish `hematite-kokoros` **only** when `libs/kokoros/` source changed.
- Publish `hematite-cli` on every tagged release after CI is green on all platforms.
- Never publish from a state where CI is red on any platform — not even one.
- Publish order: `hematite-kokoros` first (if needed), then `hematite-cli`.

```powershell
# Voice crate (only if libs/kokoros/ changed)
cargo publish -p hematite-kokoros

# Main crate (every release, after CI green)
cargo publish -p hematite-cli
```

---

## 8. Commit Style

Lowercase conventional commits only. No co-author lines.

```
feat: add X
fix: correct Y
refactor: restructure Z
chore: bump version to X.Y.Z
docs: update README
ci: fix cache key in windows workflow
```

---

## 9. Cleanup

```powershell
powershell -ExecutionPolicy Bypass -File clean.ps1                       # ghost, scratch, memories, sandbox, reports, logs
powershell -ExecutionPolicy Bypass -File clean.ps1 -Deep                 # + target/, onnx_lib/, vein.db
powershell -ExecutionPolicy Bypass -File clean.ps1 -Deep -PruneDist      # + old dist/ artifacts (keeps current version)
powershell -ExecutionPolicy Bypass -File clean.ps1 -Reset                # + PLAN.md, TASK.md (full blank-slate)
```
