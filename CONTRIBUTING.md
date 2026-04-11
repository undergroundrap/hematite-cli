# Contributing to Hematite

Hematite is a local coding harness built in Rust and designed to work with local models through LM Studio. Contribute with small, clear changes that improve real behavior on a developer machine.

## Development Principles

- Keep Hematite local-first. Core workflows should not depend on the cloud.
- Preserve Windows correctness. PowerShell, terminal behavior, and path safety matter here.
- Treat the TUI as one interface layer of the product, not the whole product.
- Prefer concrete wording over dramatic or vague phrasing in prompts, labels, and docs.
- Keep the product boundary honest: Hematite is the harness, LM Studio is the model runtime.

## Getting Started

```powershell
cargo build
cargo run
cargo run -- --no-splash
pwsh ./scripts/package-windows.ps1
```

Requirements:

- Rust toolchain
- LM Studio running locally with a model loaded on port `1234`
- Inno Setup if you want to build the Windows installer

## Project Areas

- `src/agent/`: prompting, orchestration, conversation flow, MCP, compaction, LSP, model interaction
- `src/tools/`: local tool implementations and tool registration
- `src/ui/`: TUI, voice integration, GPU monitor, and review flows
- `src/memory/`: local retrieval and session memory systems
- `libs/kokoros/`: vendored voice synthesis library

For the current module boundaries inside `src/agent/`, read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Adding a New Tool

1. Add the implementation under `src/tools/`.
2. Register the tool in the registry.
3. Respect the existing workspace safety checks for any file or shell access.
4. Keep tool output concise and machine-usable.
5. Verify the tool in a real Hematite run, not just by inspection.

If a tool changes files or shells out, assume it needs careful review.

## Editing Standards

- Prefer small, reviewable commits.
- Use conventional commit prefixes like `feat:`, `fix:`, `docs:`, and `refactor:`.
- Do not weaken path safety or approval behavior without a strong reason.
- Avoid UI labels that sound theatrical. Labels should describe what Hematite is actually doing.
- Update docs when behavior changes in a user-visible way.
- If a change adds or meaningfully changes behavior, add or update diagnostics coverage in `tests/diagnostics.rs` unless there is a clear reason not to.

## Verification

At minimum, run:

```powershell
cargo check
```

`cargo check` confirms the build graph is healthy but does not link the binary. It is not sufficient for release verification. Before cutting a release, run the full release build:

```powershell
pwsh ./scripts/package-windows.ps1
```

This is the real gate — it runs `cargo build --release` and proves the binary links and packages correctly. The portable zip in `dist/windows/` must be newer than the commit you are tagging.

For the solo Codex workflow, the practical order is: implement the change, add or update diagnostics, rebuild the local portable, test the live behavior, commit the feature work, then run the release flow.

If your change affects packaging or release behavior, also run:

```powershell
pwsh ./scripts/package-windows.ps1
```

If your change affects installer behavior, run:

```powershell
pwsh ./scripts/package-windows.ps1 -Installer
```

For behavior regressions and prompt-quality checks, use the benchmark prompts under `evals/`. Run `evals/quick_smoke.md` for fast iteration and use `evals/prompt_suite.json` plus `evals/score_template.csv` for broader manual eval runs.

## Versioning and Releases

- Package version comes from `Cargo.toml` — see `CLAUDE.md` for the full versioning policy
- For unreleased work, test the rebuilt local portable first with `pwsh ./scripts/package-windows.ps1 -AddToPath`
- When the behavior is proven, cut the release with `pwsh ./release.ps1 -Version X.Y.Z -AddToPath -Push` or the right `-Bump` variant from a clean tree
- If you need the manual path, always bump the version with `pwsh ./bump-version.ps1 -Version X.Y.Z` before the public release commit — never edit version strings by hand
- Local Windows release artifacts are built with `pwsh ./scripts/package-windows.ps1`
- Add `-Installer` for the Inno Setup installer, `-AddToPath` to register in user PATH
- Pushing a tag (`git push origin vX.Y.Z`) triggers CI automatically — `windows-release.yml` builds the Windows portable zip and installer, `unix-release.yml` builds Linux and macOS archives; both attach artifacts to the GitHub Release when they go green

Before a public release, validate the change in the local portable first, commit the feature work, then bump/tag/push from the clean known-good state. Do not manually upload release artifacts — CI handles that.
