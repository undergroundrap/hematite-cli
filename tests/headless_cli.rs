//! Headless CLI smoke matrix — Windows only.
//!
//! Exercises each primary headless flag family with every `--report-format`
//! value, asserting structural validity of the output.
//!
//! Run the normal (fast) tests:
//!   cargo test --test headless_cli
//!
//! Include the slow `--diagnose` tests:
//!   cargo test --test headless_cli -- --include-ignored

#![cfg(target_os = "windows")]

use std::path::PathBuf;
use std::process::Command;

// ── Binary path ───────────────────────────────────────────────────────────────

fn hematite() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_hematite"))
}

// ── Harness helpers ───────────────────────────────────────────────────────────

/// Run `hematite` with `args` plus `--output <tmpfile>`.
///
/// Returns the file contents. Falls back to stdout if the command printed
/// there instead (some flags write to stdout when no `--output` is given,
/// even though they respect `--output` when it is).
///
/// Panics with a diagnostic if the binary is not found or no output is produced.
fn run(args: &[&str]) -> String {
    let tmp = tempfile::tempdir().expect("temp dir");
    let out_path = tmp.path().join("out");
    let out_str = out_path.to_str().expect("utf-8 path");

    let child = Command::new(hematite())
        .args(args)
        .args(["--output", out_str])
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn hematite: {e}"));

    // Prefer the output file; fall back to stdout.
    let file = std::fs::read_to_string(&out_path).unwrap_or_default();
    let stdout = String::from_utf8_lossy(&child.stdout).into_owned();
    let content = if file.is_empty() { stdout } else { file };

    // tmp is dropped here — the file was already read into `content`.
    drop(tmp);

    assert!(
        !content.is_empty(),
        "hematite {} produced empty output\nstderr: {}",
        args.join(" "),
        String::from_utf8_lossy(&child.stderr)
            .chars()
            .take(500)
            .collect::<String>()
    );

    content
}

/// Assert content is a top-level JSON object or array (structural smoke check).
fn assert_json(content: &str, ctx: &str) {
    let t = content.trim();
    assert!(
        (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']')),
        "{ctx}: expected JSON object/array, got:\n{:.300}",
        content
    );
}

/// Assert every non-empty line of an NDJSON stream is a JSON object.
fn assert_ndjson(content: &str, ctx: &str) {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!lines.is_empty(), "{ctx}: expected at least one NDJSON line");
    for line in &lines {
        let t = line.trim();
        assert!(
            t.starts_with('{'),
            "{ctx}: NDJSON line is not a JSON object: {:.200}",
            t
        );
    }
}

/// Assert content contains the HTML5 doctype declaration.
fn assert_html(content: &str, ctx: &str) {
    assert!(
        content.to_ascii_lowercase().contains("<!doctype html>"),
        "{ctx}: HTML output missing `<!DOCTYPE html>`:\n{:.300}",
        content
    );
}

// ── --inspect summary ─────────────────────────────────────────────────────────

#[test]
fn smoke_inspect_summary_md() {
    let c = run(&["--inspect", "summary", "--report-format", "md"]);
    assert!(!c.trim().is_empty(), "--inspect summary md must be non-empty");
}

#[test]
fn smoke_inspect_summary_json() {
    let c = run(&["--inspect", "summary", "--report-format", "json"]);
    assert_json(&c, "--inspect summary json");
}

#[test]
fn smoke_inspect_summary_html() {
    let c = run(&["--inspect", "summary", "--report-format", "html"]);
    assert_html(&c, "--inspect summary html");
}

// ── --query ───────────────────────────────────────────────────────────────────

#[test]
fn smoke_query_slow_pc_md() {
    let c = run(&["--query", "why is my PC slow", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
fn smoke_query_slow_pc_json() {
    let c = run(&["--query", "why is my PC slow", "--report-format", "json"]);
    assert_json(&c, "--query json");
}

#[test]
fn smoke_query_slow_pc_html() {
    let c = run(&["--query", "why is my PC slow", "--report-format", "html"]);
    assert_html(&c, "--query html");
}

// ── --fix-all --dry-run ───────────────────────────────────────────────────────

#[test]
fn smoke_fix_all_dry_run_md() {
    let c = run(&["--fix-all", "--dry-run", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
fn smoke_fix_all_dry_run_json() {
    let c = run(&["--fix-all", "--dry-run", "--report-format", "json"]);
    assert_json(&c, "--fix-all --dry-run json");
}

#[test]
fn smoke_fix_all_dry_run_html() {
    let c = run(&["--fix-all", "--dry-run", "--report-format", "html"]);
    assert_html(&c, "--fix-all --dry-run html");
}

// ── --triage ──────────────────────────────────────────────────────────────────

#[test]
fn smoke_triage_md() {
    let c = run(&["--triage", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
fn smoke_triage_json() {
    let c = run(&["--triage", "--report-format", "json"]);
    assert_json(&c, "--triage json");
}

#[test]
fn smoke_triage_html() {
    let c = run(&["--triage", "--report-format", "html"]);
    assert_html(&c, "--triage html");
}

// ── --fix "slow" ──────────────────────────────────────────────────────────────

#[test]
fn smoke_fix_slow_md() {
    let c = run(&["--fix", "slow", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
fn smoke_fix_slow_json() {
    let c = run(&["--fix", "slow", "--report-format", "json"]);
    assert_json(&c, "--fix slow json");
}

#[test]
fn smoke_fix_slow_html() {
    let c = run(&["--fix", "slow", "--report-format", "html"]);
    assert_html(&c, "--fix slow html");
}

// ── --report ──────────────────────────────────────────────────────────────────

#[test]
fn smoke_report_md() {
    let c = run(&["--report", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
fn smoke_report_json() {
    let c = run(&["--report", "--report-format", "json"]);
    assert_json(&c, "--report json");
}

#[test]
fn smoke_report_html() {
    let c = run(&["--report", "--report-format", "html"]);
    assert_html(&c, "--report html");
}

// ── --watch resource_load --count 1 ──────────────────────────────────────────
//
// `--watch` with `--output` appends each cycle to the given file.
// `--count 1` stops after exactly one poll cycle.
// `--watch-interval 1` minimises test duration (default is 5 s).

#[test]
fn smoke_watch_resource_load_json() {
    // JSON mode: NDJSON with one object per cycle.
    let c = run(&[
        "--watch",
        "resource_load",
        "--count",
        "1",
        "--watch-interval",
        "1",
        "--report-format",
        "json",
    ]);
    assert_ndjson(&c, "--watch resource_load json");
}

#[test]
fn smoke_watch_resource_load_md() {
    // Plain-text / markdown mode: timestamped cycle block.
    let c = run(&[
        "--watch",
        "resource_load",
        "--count",
        "1",
        "--watch-interval",
        "1",
        "--report-format",
        "md",
    ]);
    assert!(!c.trim().is_empty(), "--watch md output must be non-empty");
}

// ── --diagnose (slow — staged multi-phase triage, 30–90 s) ───────────────────
//
// Marked #[ignore] so `cargo test` finishes quickly.
// Run with: cargo test --test headless_cli -- --include-ignored

#[test]
#[ignore = "staged triage takes 30–90 s; run with `-- --include-ignored`"]
fn smoke_diagnose_md() {
    let c = run(&["--diagnose", "--report-format", "md"]);
    assert!(!c.trim().is_empty());
}

#[test]
#[ignore = "staged triage takes 30–90 s; run with `-- --include-ignored`"]
fn smoke_diagnose_json() {
    let c = run(&["--diagnose", "--report-format", "json"]);
    assert_json(&c, "--diagnose json");
}

#[test]
#[ignore = "staged triage takes 30–90 s; run with `-- --include-ignored`"]
fn smoke_diagnose_html() {
    let c = run(&["--diagnose", "--report-format", "html"]);
    assert_html(&c, "--diagnose html");
}
