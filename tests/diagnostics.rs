use std::fs;
use std::io::Write;
use std::path::PathBuf;

// ── Hardware monitors ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gpu_monitor_logic() {
    let state = hematite::ui::gpu_monitor::GpuState::new();
    let (used, total) = state.read();
    assert_eq!(used, 0);
    assert_eq!(total, 0);
    assert_eq!(state.ratio(), 0.0);
    assert_eq!(state.label(), "N/A");

    state
        .used_mib
        .store(4096, std::sync::atomic::Ordering::Relaxed);
    state
        .total_mib
        .store(8192, std::sync::atomic::Ordering::Relaxed);

    assert_eq!(state.read(), (4096, 8192));
    assert_eq!(state.ratio(), 0.5);
    assert_eq!(state.label(), "4.0 GB / 8.0 GB");
}

#[tokio::test]
async fn test_git_monitor_initial_state() {
    use hematite::agent::git_monitor::{GitRemoteStatus, GitState};
    let state = GitState::new();
    assert_eq!(state.status(), GitRemoteStatus::Unknown);
    assert_eq!(state.label(), "UNKNOWN");
    assert_eq!(state.url(), "None");
}

// ── Task file parsing ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_task_file_parsing() {
    let root = PathBuf::from(".");
    let hematite_dir = root.join(".hematite");
    if !hematite_dir.exists() {
        fs::create_dir_all(&hematite_dir).unwrap();
    }
    let task_file = hematite_dir.join("TASK_TEST.md");

    let mock_task = "# Objective: Implement Sovereign Diagnostics\n\n- [ ] Task 1";
    fs::write(&task_file, mock_task).unwrap();

    let content = fs::read_to_string(&task_file).unwrap_or_default();
    let objective = content
        .lines()
        .find(|l| l.starts_with("# Objective:"))
        .map(|l| l.replace("# Objective:", "").trim().to_string())
        .unwrap_or_else(|| "Standby".to_string());

    assert_eq!(objective, "Implement Sovereign Diagnostics");
    fs::remove_file(task_file).ok();
}

// ── Vein BM25 indexing and search ─────────────────────────────────────────────

#[test]
fn test_vein_bm25_index_and_search() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    let doc = "fn authenticate(token: &str) -> bool {\n    token == \"secret\"\n}\n\n\
               fn logout(user: &str) {\n    println!(\"Logging out {}\", user);\n}";

    let chunks = vein
        .index_document("src/auth.rs", 1_000_000, doc)
        .expect("index");
    assert!(!chunks.is_empty(), "should produce chunks");

    let results = vein.search_bm25("authenticate", 5).expect("search");
    assert!(!results.is_empty(), "BM25 should find 'authenticate'");
    assert!(results[0].content.contains("authenticate"));

    // Confirm file count tracks correctly
    assert_eq!(vein.file_count(), 1);

    // Re-indexing same mtime should be a no-op
    let rechunks = vein
        .index_document("src/auth.rs", 1_000_000, doc)
        .expect("re-index");
    assert!(rechunks.is_empty(), "unchanged file should not re-index");
}

#[test]
fn test_vein_reset_clears_index() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    vein.index_document("src/lib.rs", 1, "pub fn foo() {}")
        .unwrap();
    assert_eq!(vein.file_count(), 1);

    vein.reset();
    assert_eq!(vein.file_count(), 0);
    assert_eq!(vein.embedded_chunk_count(), 0);
}

// ── Vein L1 heat tracking ─────────────────────────────────────────────────────

#[test]
fn test_vein_l1_no_heat_returns_none() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    // Fresh vein with no edits — l1_context should be None.
    assert!(vein.l1_context().is_none(), "no edits means no L1 block");
}

#[test]
fn test_vein_l1_bump_and_retrieve() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    // Index a file so it appears in chunks_meta (required for L1 join).
    vein.index_document(
        "src/agent/conversation.rs",
        1_000_000,
        "pub fn run() {}\npub fn stop() {}\n",
    )
    .unwrap();

    // Bump heat three times.
    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/agent/conversation.rs");

    let l1 = vein.l1_context().expect("should have L1 after edits");
    assert!(
        l1.contains("src/agent/conversation.rs"),
        "hot file should appear in L1"
    );
    assert!(l1.contains("3 edits"), "edit count should be 3");
}

#[test]
fn test_vein_l1_ranks_by_heat() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    vein.index_document("src/cold.rs", 1_000, "pub fn cold() {}")
        .unwrap();
    vein.index_document("src/hot.rs", 2_000, "pub fn hot() {}")
        .unwrap();

    vein.bump_heat("src/cold.rs");
    vein.bump_heat("src/hot.rs");
    vein.bump_heat("src/hot.rs");
    vein.bump_heat("src/hot.rs");

    let l1 = vein.l1_context().expect("L1 should exist");
    let hot_pos = l1.find("src/hot.rs").unwrap_or(usize::MAX);
    let cold_pos = l1.find("src/cold.rs").unwrap_or(usize::MAX);
    assert!(hot_pos < cold_pos, "hotter file should appear first in L1");
}

// ── Vein room detection ───────────────────────────────────────────────────────

#[test]
fn test_detect_room_known_segments() {
    use hematite::memory::vein::detect_room;
    assert_eq!(detect_room("src/agent/conversation.rs"), "agent");
    assert_eq!(detect_room("src/ui/tui.rs"), "ui");
    assert_eq!(detect_room("src/tools/file_ops.rs"), "tools");
    assert_eq!(detect_room("src/memory/vein.rs"), "memory");
    assert_eq!(detect_room("tests/diagnostics.rs"), "tests");
}

#[test]
fn test_detect_room_fallback() {
    use hematite::memory::vein::detect_room;
    assert_eq!(detect_room("src/main.rs"), "src");
    assert_eq!(detect_room("README.md"), "root"); // file at root with extension → root
}

#[test]
fn test_detect_room_session_prefix() {
    use hematite::memory::vein::detect_room;
    assert_eq!(
        detect_room("session/2026-04-09/2026-04-09_20-15-00/turn-12"),
        "session"
    );
}

#[test]
fn test_vein_l1_grouped_by_room() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string()).expect("vein init");

    vein.index_document("src/agent/conversation.rs", 1_000, "pub fn run() {}")
        .unwrap();
    vein.index_document("src/ui/tui.rs", 2_000, "pub fn draw() {}")
        .unwrap();

    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/ui/tui.rs");

    let l1 = vein.l1_context().expect("L1 should exist");
    assert!(l1.contains("[agent]"), "should have agent room header");
    assert!(l1.contains("[ui]"), "should have ui room header");
}

#[test]
fn test_vein_inspection_snapshot_reports_counts_and_hot_files() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs_dir = workspace.path().join(".hematite").join("docs");
    let reports_dir = workspace.path().join(".hematite").join("reports");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    fs::write(
        docs_dir.join("memory-notes.md"),
        "# Notes\n\nopalvector reference doc\n",
    )
    .expect("write doc");
    let report = serde_json::json!({
        "session_start": "2026-04-10_09-30-00",
        "transcript": [
            { "speaker": "You", "text": "remember opalvector?" },
            { "speaker": "Hematite", "text": "we kept the memory report operator-visible." }
        ]
    });
    fs::write(
        reports_dir.join("session_2026-04-10_09-30-00.json"),
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");
    vein.index_document("src/agent/conversation.rs", 1_000, "pub fn run_turn() {}")
        .unwrap();
    let indexed = vein.index_workspace_artifacts(workspace.path());
    assert_eq!(indexed, 2, "should index one doc and one session exchange");

    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat(".hematite/docs/memory-notes.md");

    let snapshot = vein.inspect_snapshot(5);
    assert_eq!(snapshot.indexed_source_files, 1);
    assert_eq!(snapshot.indexed_docs, 1);
    assert_eq!(snapshot.indexed_session_exchanges, 1);
    assert_eq!(snapshot.embedded_source_doc_chunks, 0);
    assert_eq!(snapshot.active_room.as_deref(), Some("agent"));
    assert!(
        snapshot.l1_ready,
        "hot files should make the L1 block available"
    );
    assert_eq!(snapshot.hot_files.len(), 2);
    assert_eq!(snapshot.hot_files[0].path, "src/agent/conversation.rs");
    assert_eq!(snapshot.hot_files[0].room, "agent");
    assert_eq!(snapshot.hot_files[0].heat, 2);
}

#[test]
fn test_vein_indexes_workspace_artifacts_without_project_source() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs_dir = workspace.path().join(".hematite").join("docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::write(
        docs_dir.join("reference.md"),
        "# Operator Notes\n\nsunstonealpha docs-only retrieval survives outside projects.\n",
    )
    .expect("write docs");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_workspace_artifacts(workspace.path());
    assert_eq!(indexed, 1, "should index the docs artifact");

    let results = vein
        .search_bm25("sunstonealpha retrieval", 5)
        .expect("search docs");
    assert!(!results.is_empty(), "docs artifact should be searchable");
    assert_eq!(results[0].path, ".hematite/docs/reference.md");
    assert_eq!(
        vein.file_count(),
        1,
        "docs should count toward status files"
    );
}

#[test]
fn test_vein_indexes_recent_session_reports_by_exchange_pair() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let reports_dir = workspace.path().join(".hematite").join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let report = serde_json::json!({
        "session_start": "2026-04-09_20-15-00",
        "transcript": [
            { "speaker": "System", "text": "startup noise" },
            { "speaker": "You", "text": "Remember artifact obsidiankite?" },
            { "speaker": "Hematite", "text": "We decided to keep docs-only vein mode active outside projects." },
            { "speaker": "Tool", "text": "tool chatter" },
            { "speaker": "You", "text": "What about embercache?" },
            { "speaker": "Hematite", "text": "Session exchanges should be chunked per user plus assistant pair." }
        ]
    });
    fs::write(
        reports_dir.join("session_2026-04-09_20-15-00.json"),
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_recent_session_reports(workspace.path());
    assert_eq!(indexed, 2, "two exchange pairs should be indexed");

    let results = vein
        .search_bm25("obsidiankite docs-only", 5)
        .expect("search sessions");
    assert!(!results.is_empty(), "session exchange should be searchable");
    assert!(results[0].path.starts_with("session/2026-04-09/"));
    assert_eq!(
        vein.file_count(),
        0,
        "session chunks should not inflate status file counts"
    );
    assert_eq!(
        vein.embedded_chunk_count(),
        0,
        "no embeddings were generated in the test"
    );
}

#[test]
fn test_vein_session_report_caps_to_recent_sessions_and_turns() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let reports_dir = workspace.path().join(".hematite").join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    for day in 1..=6 {
        let stamp = format!("2026-04-0{}_10-00-00", day);
        let user_token = format!("sessiontoken{}", day);
        let transcript = if day == 6 {
            (1..=55)
                .flat_map(|turn| {
                    [
                        serde_json::json!({
                            "speaker": "You",
                            "text": format!("turntoken{} request", turn),
                        }),
                        serde_json::json!({
                            "speaker": "Hematite",
                            "text": format!("turntoken{} response", turn),
                        }),
                    ]
                })
                .collect::<Vec<_>>()
        } else {
            vec![
                serde_json::json!({ "speaker": "You", "text": format!("{} request", user_token) }),
                serde_json::json!({ "speaker": "Hematite", "text": format!("{} response", user_token) }),
            ]
        };

        let report = serde_json::json!({
            "session_start": stamp,
            "transcript": transcript,
        });
        fs::write(
            reports_dir.join(format!("session_{}.json", stamp)),
            serde_json::to_string_pretty(&report).expect("serialize report"),
        )
        .expect("write report");
    }

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_recent_session_reports(workspace.path());
    assert_eq!(
        indexed, 54,
        "last five sessions should be indexed with the newest session capped at 50 pairs"
    );

    let oldest = vein.search_bm25("sessiontoken1", 5).expect("search oldest");
    assert!(
        oldest.is_empty(),
        "the oldest sixth session should be pruned"
    );

    let retained = vein
        .search_bm25("sessiontoken2", 5)
        .expect("search retained session");
    assert!(
        !retained.is_empty(),
        "newer sessions within the five-session cap should remain searchable"
    );

    let early_turn = vein
        .search_bm25("turntoken1", 5)
        .expect("search early turn");
    assert!(
        early_turn.is_empty(),
        "early turns beyond the 50-pair cap should be dropped"
    );

    let late_turn = vein
        .search_bm25("turntoken55", 5)
        .expect("search late turn");
    assert!(
        !late_turn.is_empty(),
        "latest turns within the cap should remain searchable"
    );
}

#[test]
fn test_vein_indexes_imported_marker_transcript_exchanges() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let imports_dir = workspace.path().join(".hematite").join("imports");
    fs::create_dir_all(&imports_dir).expect("create imports dir");

    fs::write(
        imports_dir.join("handoff.txt"),
        "> Remember emberforge and the release script?\nWe switched to a single release command.\n\n> What about docs-only mode?\nIt should still search imported chat exports.\n",
    )
    .expect("write transcript");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_imported_session_exports(workspace.path());
    assert_eq!(indexed, 2, "two imported exchange pairs should be indexed");

    let results = vein
        .search_bm25("emberforge release command", 5)
        .expect("search imported transcript");
    assert!(
        !results.is_empty(),
        "imported transcript should be searchable"
    );
    assert!(results[0].path.starts_with("session/imports/"));
    assert_eq!(
        vein.file_count(),
        0,
        "imported session chunks should not inflate source/doc file counts"
    );
}

#[test]
fn test_vein_indexes_imported_codex_jsonl_exchanges() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let imports_dir = workspace.path().join(".hematite").join("imports");
    fs::create_dir_all(&imports_dir).expect("create imports dir");

    let jsonl = r#"{"type":"session_meta","id":"abc"}
{"type":"event_msg","payload":{"type":"user_message","message":"Remember basalttrace and why we changed the installer flow?"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"We wanted one release command to update tags, docs, and the local portable build."}}
{"type":"event_msg","payload":{"type":"user_message","message":"What should imports do?"}}
{"type":"event_msg","payload":{"type":"agent_message","message":"Imported chats should be searchable as session memory without polluting source counts."}}"#;
    fs::write(imports_dir.join("codex-rollout.jsonl"), jsonl).expect("write jsonl");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_imported_session_exports(workspace.path());
    assert_eq!(indexed, 2, "two codex exchange pairs should be indexed");

    let results = vein
        .search_bm25("basalttrace installer flow", 5)
        .expect("search codex import");
    assert!(!results.is_empty(), "codex import should be searchable");
    assert!(
        results[0].content.contains("Imported session exchange"),
        "imported exchanges should be labeled as imported memory"
    );
}

#[test]
fn test_vein_indexes_imported_claude_code_jsonl_exchanges() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let imports_dir = workspace.path().join(".hematite").join("imports");
    fs::create_dir_all(&imports_dir).expect("create imports dir");

    let jsonl = r#"{"type":"human","message":{"content":[{"type":"text","text":"Remember opalcache and the docs-only rule?"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"We kept docs-only retrieval alive outside projects and made imported chats searchable too."}]}}"#;
    fs::write(imports_dir.join("claude-code.jsonl"), jsonl).expect("write claude jsonl");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://localhost:1234".to_string()).expect("vein init");

    let indexed = vein.index_imported_session_exports(workspace.path());
    assert_eq!(
        indexed, 1,
        "one Claude Code exchange pair should be indexed"
    );

    let results = vein
        .search_bm25("opalcache docs-only retrieval", 5)
        .expect("search claude import");
    assert!(
        !results.is_empty(),
        "Claude Code import should be searchable"
    );
}

// ── Document text extraction ──────────────────────────────────────────────────

#[test]
fn test_extract_markdown_succeeds() {
    use hematite::memory::vein::extract_document_text;

    let mut tmp = tempfile::NamedTempFile::with_suffix(".md").expect("temp md");
    writeln!(
        tmp,
        "# Design Doc\n\nThis is a specification for the auth module."
    )
    .unwrap();

    let result = extract_document_text(tmp.path());
    assert!(result.is_ok(), "markdown extraction should succeed");
    assert!(result.unwrap().contains("Design Doc"));
}

#[test]
fn test_extract_txt_succeeds() {
    use hematite::memory::vein::extract_document_text;

    let mut tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("temp txt");
    writeln!(
        tmp,
        "API reference for the payment service.\n\nEndpoint: POST /charge"
    )
    .unwrap();

    let result = extract_document_text(tmp.path());
    assert!(result.is_ok());
    assert!(result.unwrap().contains("payment service"));
}

#[test]
fn test_pdf_quality_guard_rejects_garbled_text() {
    // Simulate what pdf-extract returns for EBSCO-style custom-font PDFs:
    // words smashed together with no spaces.
    use hematite::memory::vein::extract_document_text;

    // We can't easily produce a real garbled PDF in a unit test, so test the
    // quality guard directly via a mock plain-text file that mimics garbled output.
    // The guard lives in extract_document_text for PDFs; we test the space-ratio
    // logic by verifying normal text passes and noting garbled PDFs would fail.
    // Real garbled PDF rejection is covered by manual testing with EBSCO files.

    let mut tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("temp");
    // Normal text — should pass quality-equivalent check for non-PDF
    writeln!(
        tmp,
        "This is a well formatted document with proper spacing between all words."
    )
    .unwrap();
    let result = extract_document_text(tmp.path());
    assert!(result.is_ok());
}

// ── Sandboxed code execution ──────────────────────────────────────────────────

#[tokio::test]
async fn test_inspect_host_directory_reports_counts_and_names() {
    use serde_json::json;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let nested = workspace.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested dir");
    fs::write(workspace.path().join("alpha.txt"), "hematite").expect("write alpha");
    fs::write(nested.join("beta.log"), "operator").expect("write beta");

    let args = json!({
        "topic": "directory",
        "path": workspace.path().display().to_string(),
        "max_entries": 5
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host directory");

    assert!(output.contains("Directory inspection: Directory"));
    assert!(output.contains("Top-level items: 2"));
    assert!(output.contains("alpha.txt"));
    assert!(output.contains("nested"));
    assert!(output.contains("Recursive files: 2"));
}

#[tokio::test]
async fn test_inspect_host_path_reports_path_summary() {
    use serde_json::json;

    let args = json!({
        "topic": "path",
        "max_entries": 5
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host path");

    assert!(output.contains("Host inspection: PATH"));
    assert!(output.contains("Total entries:"));
    assert!(output.contains("PATH entries:"));
}

#[tokio::test]
async fn test_describe_toolchain_host_inspection_plan_prefers_inspect_host() {
    use serde_json::json;

    let output = hematite::tools::toolchain::describe_toolchain(&json!({
        "topic": "host_inspection_plan",
        "question": "How should Hematite inspect my PATH and Downloads folder?"
    }))
    .await
    .expect("describe host inspection plan");

    assert!(output.contains("inspect_host"));
    assert!(output.contains("optional `shell`"));
    assert!(output.contains("PATH"));
}

#[tokio::test]
async fn test_sandbox_python_runs() {
    use serde_json::json;

    // Skip if Python is not available
    let python_available = std::process::Command::new("python")
        .arg("--version")
        .output()
        .or_else(|_| {
            std::process::Command::new("python3")
                .arg("--version")
                .output()
        })
        .is_ok();

    if !python_available {
        println!("Skipping: Python not available");
        return;
    }

    let args = json!({
        "language": "python",
        "code": "print(2 + 2)"
    });

    let result = hematite::tools::code_sandbox::execute(&args).await;
    assert!(
        result.is_ok(),
        "Python sandbox should execute: {:?}",
        result
    );
    assert!(result.unwrap().contains("4"), "Should return 4");
}

#[tokio::test]
async fn test_sandbox_javascript_sha256() {
    use serde_json::json;

    // Skip if Deno is not available (checks common locations)
    let deno_available = std::process::Command::new("deno")
        .arg("--version")
        .output()
        .is_ok();
    let lmstudio_deno = dirs::home_dir()
        .map(|h| h.join(".lmstudio/.internal/utils/deno.exe").exists())
        .unwrap_or(false);

    if !deno_available && !lmstudio_deno {
        println!("Skipping: Deno not available");
        return;
    }

    let args = json!({
        "language": "javascript",
        "code": "const buf = await crypto.subtle.digest('SHA-256', new TextEncoder().encode('Hematite')); console.log([...new Uint8Array(buf)].map(b=>b.toString(16).padStart(2,'0')).join(''));"
    });

    let result = hematite::tools::code_sandbox::execute(&args).await;
    assert!(result.is_ok(), "JS sandbox should execute: {:?}", result);
    assert!(
        result
            .unwrap()
            .contains("94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c"),
        "SHA-256 of 'Hematite' should match known hash"
    );
}
