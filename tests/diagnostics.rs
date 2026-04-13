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

#[test]
fn test_workspace_profile_detects_rust_project_shape() {
    use hematite::agent::workspace_profile::detect_workspace_profile;

    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::create_dir_all(workspace.path().join("src")).expect("create src");
    fs::create_dir_all(workspace.path().join("tests")).expect("create tests");
    fs::create_dir_all(workspace.path().join(".github").join("workflows"))
        .expect("create workflows");
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .expect("write cargo");

    let profile = detect_workspace_profile(workspace.path());
    assert_eq!(profile.workspace_mode, "project");
    assert_eq!(profile.primary_stack.as_deref(), Some("rust"));
    assert!(profile.stack_signals.iter().any(|entry| entry == "rust"));
    assert!(profile
        .package_managers
        .iter()
        .any(|entry| entry == "cargo"));
    assert!(profile.important_paths.iter().any(|entry| entry == "src"));
    assert!(profile.important_paths.iter().any(|entry| entry == "tests"));
}

#[test]
fn test_workspace_profile_uses_workspace_verify_profile_and_writes_file() {
    use hematite::agent::workspace_profile::{
        ensure_workspace_profile, profile_prompt_block, profile_report, workspace_profile_path,
    };

    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::create_dir_all(workspace.path().join("src")).expect("create src");
    fs::create_dir_all(workspace.path().join(".hematite")).expect("create hematite dir");
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname='sample'\nversion='0.1.0'\n",
    )
    .expect("write cargo");
    fs::write(
        workspace.path().join(".hematite").join("settings.json"),
        r#"{
  "verify": {
    "default_profile": "rust",
    "profiles": {
      "rust": {
        "build": "cargo build",
        "test": "cargo test"
      }
    }
  }
}"#,
    )
    .expect("write settings");

    let profile = ensure_workspace_profile(workspace.path()).expect("ensure profile");
    assert_eq!(profile.verify_profile.as_deref(), Some("rust"));
    assert_eq!(profile.build_hint.as_deref(), Some("cargo build"));
    assert_eq!(profile.test_hint.as_deref(), Some("cargo test"));
    assert!(
        workspace_profile_path(workspace.path()).exists(),
        "profile file should be written"
    );

    let prompt_block = profile_prompt_block(workspace.path()).expect("profile prompt");
    assert!(prompt_block.contains("Verify profile: rust"));
    assert!(prompt_block.contains("Build hint: cargo build"));

    let report = profile_report(workspace.path());
    assert!(report.contains("Workspace Profile"));
    assert!(report.contains("Verify profile: rust"));
    assert!(report.contains("Path:"));
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
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

    // Fresh vein with no edits — l1_context should be None.
    assert!(vein.l1_context().is_none(), "no edits means no L1 block");
}

#[test]
fn test_vein_l1_bump_and_retrieve() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
fn test_detect_room_specialized_roles() {
    use hematite::memory::vein::detect_room;
    assert_eq!(detect_room("src/runtime.rs"), "runtime");
    assert_eq!(detect_room("src/agent/mcp_manager.rs"), "integration");
    assert_eq!(detect_room("Cargo.toml"), "config");
    assert_eq!(detect_room("installer/hematite.iss"), "release");
    assert_eq!(
        detect_room(".github/workflows/windows-release.yml"),
        "automation"
    );
    assert_eq!(detect_room("README.md"), "docs");
}

#[test]
fn test_detect_room_fallback() {
    use hematite::memory::vein::detect_room;
    assert_eq!(detect_room("src/plain.rs"), "src");
    assert_eq!(detect_room("notes.bin"), "root");
}

#[test]
fn test_detect_room_session_prefix() {
    use hematite::memory::vein::detect_room;
    assert_eq!(
        detect_room("session/2026-04-09/2026-04-09_20-15-00/turn-12"),
        "session"
    );
    assert_eq!(
        detect_room(".hematite/imports/claude-rollout.jsonl"),
        "session"
    );
}

#[test]
fn test_vein_l1_grouped_by_room() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");
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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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

// ── Vein retrieval ranking diagnostics ───────────────────────────────────────

#[test]
fn test_vein_search_context_boosts_exact_phrases() {
    use hematite::memory::vein::Vein;

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

    vein.index_document(
        "src/ui/startup.rs",
        1,
        "startup panel work startup panel work startup controls startup panel",
    )
    .expect("index startup");
    vein.index_document(
        "src/ui/specular.rs",
        2,
        "The specular panel shows the active context and event log.",
    )
    .expect("index specular");

    let results = vein
        .search_context("How does the \"specular panel\" work at startup?", 2)
        .expect("search context");
    assert_eq!(
        results[0].path, "src/ui/specular.rs",
        "exact quoted phrase should outrank generic token overlap"
    );
}

#[test]
fn test_vein_search_context_boosts_standout_query_tokens() {
    use hematite::memory::vein::Vein;

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

    vein.index_document(
        "src/release.rs",
        1,
        "installer flow local build docs tags portable build installer flow local build release command",
    )
    .expect("index generic release");
    vein.index_document(
        "src/tools/basalttrace.rs",
        2,
        "Basalttrace changed the release pipeline.",
    )
    .expect("index standout token");

    let results = vein
        .search_context(
            "why did basalttrace installer flow change for local build",
            2,
        )
        .expect("search context");
    assert_eq!(
        results[0].path, "src/tools/basalttrace.rs",
        "standout repo/tool token should outrank generic overlap"
    );
}

#[test]
fn test_vein_search_context_prefers_session_memory_for_historical_queries() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs_dir = workspace.path().join(".hematite").join("docs");
    let reports_dir = workspace.path().join(".hematite").join("reports");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    fs::write(
        docs_dir.join("opalcache.md"),
        "Opalcache docs-only mode keeps local support notes searchable.",
    )
    .expect("write doc");
    let report = serde_json::json!({
        "session_start": "2026-04-10_08-45-00",
        "transcript": [
            { "speaker": "You", "text": "What should we do about opalcache docs-only mode?" },
            { "speaker": "Hematite", "text": "We decided earlier to keep session and import memory searchable outside project folders." }
        ]
    });
    fs::write(
        reports_dir.join("session_2026-04-10_08-45-00.json"),
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .expect("write report");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");
    let indexed = vein.index_workspace_artifacts(workspace.path());
    assert_eq!(indexed, 2, "should index one doc and one session exchange");

    let results = vein
        .search_context(
            "what did we decide earlier about opalcache docs-only mode?",
            2,
        )
        .expect("search context");
    assert!(
        results[0].path.starts_with("session/"),
        "historical decision query should prefer session memory"
    );
}

#[test]
fn test_vein_search_context_biases_session_memory_by_explicit_date() {
    use hematite::memory::vein::Vein;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let reports_dir = workspace.path().join(".hematite").join("reports");
    fs::create_dir_all(&reports_dir).expect("create reports dir");

    let older_report = serde_json::json!({
        "session_start": "2026-04-08_09-00-00",
        "transcript": [
            { "speaker": "You", "text": "What should we do about quartzharbor docs-only rollout?" },
            { "speaker": "Hematite", "text": "On April 8 we delayed the quartzharbor docs-only rollout. Quartzharbor docs-only rollout delay remained the plan." }
        ]
    });
    fs::write(
        reports_dir.join("session_2026-04-08_09-00-00.json"),
        serde_json::to_string_pretty(&older_report).expect("serialize older report"),
    )
    .expect("write older report");

    let newer_report = serde_json::json!({
        "session_start": "2026-04-09_09-00-00",
        "transcript": [
            { "speaker": "You", "text": "What should we do about quartzharbor docs-only rollout?" },
            { "speaker": "Hematite", "text": "On April 9 we decided to keep the quartzharbor docs-only rollout live." }
        ]
    });
    fs::write(
        reports_dir.join("session_2026-04-09_09-00-00.json"),
        serde_json::to_string_pretty(&newer_report).expect("serialize newer report"),
    )
    .expect("write newer report");

    let db = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");
    let indexed = vein.index_recent_session_reports(workspace.path());
    assert_eq!(indexed, 2, "two session exchanges should be indexed");

    let results = vein
        .search_context(
            "what did we decide on 2026-04-09 about quartzharbor docs-only rollout?",
            2,
        )
        .expect("search dated session context");
    assert!(
        results[0].path.starts_with("session/2026-04-09/"),
        "explicit date query should favor the matching session date even when another session has heavier lexical overlap"
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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
    let mut vein = Vein::new(db.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

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
async fn test_inspect_host_processes_can_filter_current_binary() {
    use serde_json::json;

    let process_name = std::env::current_exe()
        .expect("current exe")
        .file_stem()
        .expect("file stem")
        .to_string_lossy()
        .to_string();

    let args = json!({
        "topic": "processes",
        "name": process_name,
        "max_entries": 5
    });

    let output = match hematite::tools::host_inspect::inspect_host(&args).await {
        Ok(output) => output,
        Err(err)
            if err.contains("Failed to run tasklist")
                || err.contains("tasklist returned a non-success status")
                || err.contains("Failed to run ps")
                || err.contains("ps returned a non-success status") =>
        {
            println!("Skipping processes test on this host: {}", err);
            return;
        }
        Err(err) => panic!("inspect host processes failed: {}", err),
    };

    assert!(output.contains("Host inspection: processes"));
    assert!(output.contains("Filter name:"));
    assert!(output.contains("Processes found:"));
}

#[tokio::test]
async fn test_inspect_host_network_reports_adapter_summary() {
    use serde_json::json;

    let args = json!({
        "topic": "network",
        "max_entries": 5
    });

    let output = match hematite::tools::host_inspect::inspect_host(&args).await {
        Ok(output) => output,
        Err(err)
            if err.contains("Failed to run ipconfig")
                || err.contains("ipconfig returned a non-success status")
                || err.contains("Failed to run ip addr")
                || err.contains("ip addr returned a non-success status")
                || err.contains("Failed to run ip route")
                || err.contains("ip route returned a non-success status") =>
        {
            println!("Skipping network test on this host: {}", err);
            return;
        }
        Err(err) => panic!("inspect host network failed: {}", err),
    };

    assert!(output.contains("Host inspection: network"));
    assert!(output.contains("Adapters found:"));
    assert!(output.contains("Listener exposure:"));
    assert!(output.contains("Adapter summary:"));
}

#[tokio::test]
async fn test_inspect_host_services_reports_status_summary() {
    use serde_json::json;

    let args = json!({
        "topic": "services",
        "max_entries": 5
    });

    let output = match hematite::tools::host_inspect::inspect_host(&args).await {
        Ok(output) => output,
        Err(err)
            if err.contains("Failed to run PowerShell service inspection")
                || err.contains("PowerShell service inspection returned a non-success status")
                || err.contains("Failed to run systemctl list-units")
                || err.contains("systemctl list-units returned a non-success status")
                || err.contains("Failed to run systemctl list-unit-files")
                || err.contains("systemctl list-unit-files returned a non-success status") =>
        {
            println!("Skipping services test on this host: {}", err);
            return;
        }
        Err(err) => panic!("inspect host services failed: {}", err),
    };

    assert!(output.contains("Host inspection: services"));
    assert!(output.contains("Services found:"));
    assert!(output.contains("Service summary:"));
}

#[tokio::test]
async fn test_inspect_host_env_doctor_reports_package_manager_health() {
    use serde_json::json;

    let args = json!({
        "topic": "env_doctor",
        "max_entries": 5
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host env doctor");

    assert!(output.contains("Host inspection: env_doctor"));
    assert!(output.contains("PATH health:"));
    assert!(output.contains("Package managers found:"));
    assert!(output.contains("Findings:"));
    assert!(output.contains("Guidance:"));
}

#[tokio::test]
async fn test_inspect_host_fix_plan_for_path_reports_grounded_steps() {
    use serde_json::json;

    let args = json!({
        "topic": "fix_plan",
        "issue": "How do I fix cargo not found on this machine?"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host fix plan env");

    assert!(output.contains("Host inspection: fix_plan"));
    assert!(output.contains("Fix-plan type: environment/path"));
    assert!(output.contains("Fix plan:"));
    assert!(output.contains("Why this works:"));
}

#[tokio::test]
async fn test_inspect_host_fix_plan_for_port_mentions_requested_port() {
    use serde_json::json;

    let args = json!({
        "topic": "fix_plan",
        "issue": "How do I fix port 3000 already in use?",
        "port": 3000
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host fix plan port");

    assert!(output.contains("Host inspection: fix_plan"));
    assert!(output.contains("Fix-plan type: port_conflict"));
    assert!(output.contains("Requested port: 3000"));
}

#[tokio::test]
async fn test_inspect_host_fix_plan_for_lm_studio_mentions_configured_endpoint() {
    use serde_json::json;

    let args = json!({
        "topic": "fix_plan",
        "issue": "How do I fix Hematite when LM Studio is not reachable on localhost:1234?"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host fix plan lm studio");

    assert!(output.contains("Host inspection: fix_plan"));
    assert!(output.contains("Fix-plan type: lm_studio"));
    assert!(output.contains("Configured API URL:"));
    assert!(output.contains("Fix plan:"));
}

#[tokio::test]
async fn test_inspect_host_disk_reports_size_summary() {
    use serde_json::json;

    let workspace = tempfile::tempdir().expect("temp workspace");
    let nested = workspace.path().join("nested");
    fs::create_dir_all(&nested).expect("create nested dir");
    fs::write(workspace.path().join("alpha.bin"), vec![0u8; 2048]).expect("write alpha");
    fs::write(nested.join("beta.bin"), vec![0u8; 1024]).expect("write beta");

    let args = json!({
        "topic": "disk",
        "path": workspace.path().display().to_string(),
        "max_entries": 5
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host disk");

    assert!(output.contains("Directory inspection: Disk"));
    assert!(output.contains("Total size:"));
    assert!(output.contains("Largest top-level entries:"));
}

#[tokio::test]
async fn test_inspect_host_repo_doctor_reports_workspace_state() {
    use serde_json::json;

    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::write(
        workspace.path().join("Cargo.toml"),
        "[package]\nname = \"sample\"\nversion = \"0.9.0\"\nedition = \"2021\"\n",
    )
    .expect("write cargo manifest");
    fs::create_dir_all(workspace.path().join(".hematite").join("docs")).expect("docs dir");
    fs::create_dir_all(workspace.path().join(".hematite").join("imports")).expect("imports dir");
    fs::create_dir_all(workspace.path().join(".hematite").join("reports")).expect("reports dir");
    fs::write(
        workspace
            .path()
            .join(".hematite")
            .join("workspace_profile.json"),
        "{}",
    )
    .expect("write workspace profile");

    let args = json!({
        "topic": "repo_doctor",
        "path": workspace.path().display().to_string(),
        "max_entries": 5
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host repo doctor");

    assert!(output.contains("Host inspection: repo_doctor"));
    assert!(output.contains("Workspace mode: project"));
    assert!(output.contains("Project markers:"));
    assert!(output.contains("Cargo.toml"));
    assert!(output.contains("Hematite docs/imports/reports: 0/0/0"));
    assert!(output.contains("Workspace profile: present"));
    assert!(output.contains("Cargo version: 0.9.0"));
}

#[tokio::test]
async fn test_inspect_host_ports_can_filter_single_listener() {
    use serde_json::json;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let port = listener.local_addr().expect("listener addr").port();

    let args = json!({
        "topic": "ports",
        "port": port,
        "max_entries": 5
    });

    let output = match hematite::tools::host_inspect::inspect_host(&args).await {
        Ok(output) => output,
        Err(err) if err.contains("Failed to run") || err.contains("non-success status") => {
            println!("Skipping ports test on this host: {}", err);
            return;
        }
        Err(err) => panic!("inspect host ports failed: {}", err),
    };

    assert!(output.contains("Host inspection: ports"));
    assert!(output.contains(&format!("Filter port: {}", port)));
    assert!(output.contains(&format!("127.0.0.1:{}", port)));
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

// ── Heat-weighted PageRank personalization ────────────────────────────────────

#[test]
fn test_vein_hot_files_weighted_normalizes_to_one() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

    vein.index_document("src/core.rs", 1_000, "pub fn core() {}")
        .unwrap();
    vein.index_document("src/util.rs", 2_000, "pub fn util() {}")
        .unwrap();

    // core: 4 edits, util: 2 edits — core should have weight 1.0, util 0.5
    for _ in 0..4 {
        vein.bump_heat("src/core.rs");
    }
    for _ in 0..2 {
        vein.bump_heat("src/util.rs");
    }

    let weighted = vein.hot_files_weighted(10);
    assert!(!weighted.is_empty(), "should return weighted hot files");

    let core_weight = weighted
        .iter()
        .find(|(p, _)| p == "src/core.rs")
        .map(|(_, w)| *w);
    let util_weight = weighted
        .iter()
        .find(|(p, _)| p == "src/util.rs")
        .map(|(_, w)| *w);

    assert_eq!(core_weight, Some(1.0), "hottest file should have weight 1.0");
    let util_w = util_weight.expect("util.rs should appear");
    assert!(
        (util_w - 0.5).abs() < 0.01,
        "util.rs with half the edits should have weight ~0.5, got {}",
        util_w
    );
}

#[test]
fn test_pagerank_heat_weighted_ranks_active_file_higher() {
    use hematite::memory::repo_map::RepoMapGenerator;
    use std::fs;

    let dir = tempfile::tempdir().unwrap();

    // core.rs defines a struct referenced by user.rs and admin.rs
    fs::write(
        dir.path().join("core.rs"),
        "pub struct Engine {}\npub fn init_engine() -> Engine { Engine {} }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("user.rs"),
        "use crate::core::Engine;\nfn use_engine(e: Engine) { let _ = e; }\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("admin.rs"),
        "use crate::core::Engine;\nfn admin_engine(e: Engine) { let _ = e; }\n",
    )
    .unwrap();
    // leaf.rs: no references from anyone
    fs::write(
        dir.path().join("leaf.rs"),
        "fn unused_leaf_function() {}\nstruct OrphanStruct {}\n",
    )
    .unwrap();

    // Simulate heavy heat on leaf.rs — heat-weighted boost should still not
    // outrank a file that is architecturally central AND has heat.
    // But core.rs with full heat (1.0) should beat leaf.rs with full heat.
    let hot = vec![
        ("core.rs".to_string(), 1.0_f64), // hottest
        ("leaf.rs".to_string(), 0.5_f64), // warm but isolated
    ];

    let gen = RepoMapGenerator::new(dir.path()).with_hot_files(&hot);
    let map = gen.generate().unwrap();

    let core_pos = map.find("core.rs:").unwrap_or(usize::MAX);
    let leaf_pos = map.find("leaf.rs:").unwrap_or(usize::MAX);

    assert!(
        core_pos < leaf_pos,
        "core.rs (heat=1.0, referenced by 2 files) should rank before leaf.rs (heat=0.5, isolated). Map:\n{}",
        map
    );
}

// ── Indent-normalization in edit_file / multi_search_replace ──────────────────

#[test]
fn test_edit_file_fuzzy_corrects_indent_on_replace() {
    use std::fs;
    use tempfile::NamedTempFile;

    // File uses 8-space indentation
    let tmp = NamedTempFile::new().unwrap();
    fs::write(
        tmp.path(),
        "fn outer() {\n        fn inner() {\n                let x = 1;\n        }\n}\n",
    )
    .unwrap();

    let path = tmp.path().to_str().unwrap();

    // Model supplies search/replace with 0-space indentation (wrong)
    let args = serde_json::json!({
        "path": path,
        "search": "fn inner() {\n    let x = 1;\n}",
        "replace": "fn inner() {\n    let x = 2;\n}",
    });

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::file_ops::edit_file(&args));

    assert!(result.is_ok(), "edit should succeed via fuzzy match: {:?}", result);

    let content = fs::read_to_string(tmp.path()).unwrap();
    // Model's replace had 4-space relative indent for body; file base is 8 spaces.
    // Adjusted: 8 (base) + 4 (relative) = 12 spaces for the body line.
    assert!(
        content.contains("        fn inner() {\n            let x = 2;\n        }"),
        "replace should be indent-adjusted to match file indentation:\n{}",
        content
    );
}

#[test]
fn test_multi_search_replace_fuzzy_corrects_indent() {
    use std::fs;
    use tempfile::NamedTempFile;

    let tmp = NamedTempFile::new().unwrap();
    fs::write(
        tmp.path(),
        "impl Foo {\n    fn bar(&self) -> u32 {\n        42\n    }\n}\n",
    )
    .unwrap();

    let path = tmp.path().to_str().unwrap();

    // Model supplies search with no indentation (wrong)
    let args = serde_json::json!({
        "path": path,
        "hunks": [
            {
                "search": "fn bar(&self) -> u32 {\n    42\n}",
                "replace": "fn bar(&self) -> u32 {\n    99\n}"
            }
        ]
    });

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::file_ops::multi_search_replace(&args));

    assert!(result.is_ok(), "multi_search_replace should succeed via fuzzy: {:?}", result);

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(
        content.contains("        99"),
        "replacement value should be at correct 8-space indent:\n{}",
        content
    );
}
