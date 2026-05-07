# Hematite dev task runner
# Install: cargo install just   or   winget install Casey.Just
# Usage:   just <recipe>

# Show available recipes
default:
    @just --list

# ── Quality Gates ────────────────────────────────────────────────────────────

# Run all checks (mirrors CI exactly)
check: fmt-check clippy test audit

# Check formatting without modifying files
fmt-check:
    cargo fmt --all --check

# Format all source files
fmt:
    cargo fmt --all

# Clippy with CI-matching strictness
clippy:
    cargo clippy --all-targets -- -D warnings

# Run the full test suite
test:
    cargo test --lib
    cargo test --test diagnostics
    cargo test --test routing_precision
    cargo test --test debug_routing

# Run a single integration test by name (tab-complete friendly)
# Usage: just test-one test_routing_detects_outlook_topic
test-one name:
    cargo test --test diagnostics {{ name }} -- --exact --nocapture

# Run all lib unit tests matching a filter
# Usage: just test-unit routing
test-unit filter="":
    cargo test --lib {{ filter }}

# ── Security ─────────────────────────────────────────────────────────────────

# Check Cargo.lock for known CVEs
audit:
    cargo audit

# Full supply chain check (licenses, bans, advisories, sources)
deny:
    cargo deny check

# List dependencies with newer versions available (requires cargo-outdated)
# Install: cargo install cargo-outdated
outdated:
    cargo outdated --depth 1

# ── Build ─────────────────────────────────────────────────────────────────────

# Fast compile check (no linking)
check-build:
    cargo check --tests

# Debug build
build:
    cargo build

# Release build (slow — prefer package for real releases)
build-release:
    cargo build --release

# ── Packaging ────────────────────────────────────────────────────────────────

# Build portable Windows bundle and update PATH (primary dev loop)
package:
    pwsh ./scripts/package-windows.ps1 -AddToPath

# Build portable + Inno Setup installer
package-installer:
    pwsh ./scripts/package-windows.ps1 -Installer -AddToPath

# ── Versioning & Release ──────────────────────────────────────────────────────

# Verify version strings are in sync across all files
# Usage: just verify-version 0.8.1
verify-version version:
    pwsh ./scripts/verify-version-sync.ps1 -Version {{ version }} -RequireCargoLock

# Verify CLAUDE.md matches current codebase architecture
verify-docs:
    pwsh ./scripts/verify-doc-sync.ps1

# Cut a full release: bump, build, tag, push
# Usage: just release 0.9.0
release version:
    pwsh ./release.ps1 -Version {{ version }} -AddToPath -Push

# ── Cleanup ───────────────────────────────────────────────────────────────────

# Remove runtime artifacts (ghost, scratch, reports, logs)
clean:
    pwsh ./clean.ps1

# Deep clean: also removes target/ and vein.db (next build will be slow)
clean-deep:
    pwsh ./clean.ps1 -Deep

# Full reset: deep clean + wipe session state (simulates first-run)
clean-reset:
    pwsh ./clean.ps1 -Reset

# ── Diagnostics ───────────────────────────────────────────────────────────────

# Show test suite structure at a glance
test-info:
    @echo "Integration tests (tests/):"
    @echo "  diagnostics.rs       422 routing & inspection tests"
    @echo "  routing_precision.rs  17 topic-routing precision tests"
    @echo "  scientific.rs          5 scientific computing tests"
    @echo "  data_analysis.rs       3 data analysis tests"
    @echo "  debug_routing.rs       1 routing-collision test"
    @echo ""
    @echo "Unit tests (src/**/#[cfg(test)]):"
    @echo "  ~250 inline tests across 37 modules"
    @echo ""
    @echo "CI jobs:"
    @echo "  lint  — fmt check + cargo audit + cargo deny  (Linux, no compile)"
    @echo "  test  — clippy + all integration tests        (Windows)"
