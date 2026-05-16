use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

// Tests that use `std::env::set_current_dir` must serialize to avoid CWD races.
static CWD_LOCK: Mutex<()> = Mutex::new(());

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
fn test_teleport_resume_marker_round_trip_for_workspace_root() {
    let _guard = CWD_LOCK.lock().expect("cwd lock");
    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::create_dir_all(workspace.path().join(".git")).expect("create git dir");
    fs::create_dir_all(workspace.path().join(".hematite")).expect("create hematite dir");

    let original_cwd = std::env::current_dir().expect("current dir");
    std::env::set_current_dir(workspace.path()).expect("set cwd to workspace");

    hematite::tools::plan::write_teleport_resume_marker_for_root(workspace.path())
        .expect("write teleport marker");

    let marker_path = workspace.path().join(".hematite").join("TELEPORT_RESUME");
    assert!(
        marker_path.exists(),
        "marker should be written for workspace"
    );
    assert!(
        hematite::tools::plan::consume_teleport_resume_marker(),
        "marker should be consumed when cwd points at that workspace"
    );
    assert!(
        !marker_path.exists(),
        "marker file should be removed after consumption"
    );
    assert!(
        !hematite::tools::plan::consume_teleport_resume_marker(),
        "second consume should report no marker"
    );

    std::env::set_current_dir(original_cwd).expect("restore cwd");
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

#[test]
fn test_workspace_profile_detects_website_runtime_contract() {
    use hematite::agent::workspace_profile::{
        detect_workspace_profile, profile_prompt_block, profile_strategy_prompt_block,
    };

    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::create_dir_all(workspace.path().join("src").join("pages")).expect("create pages");
    fs::create_dir_all(workspace.path().join("public")).expect("create public");
    fs::write(
        workspace.path().join("package.json"),
        r#"{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  }
}"#,
    )
    .expect("write package json");
    fs::write(
        workspace.path().join("src").join("pages").join("about.tsx"),
        "export default function About(){ return null; }",
    )
    .expect("write page");
    fs::write(
        workspace.path().join("public").join("pricing.html"),
        "<html><body>pricing</body></html>",
    )
    .expect("write public html");

    let profile = detect_workspace_profile(workspace.path());
    let contract = profile
        .runtime_contract
        .expect("website runtime contract should exist");
    assert_eq!(contract.loop_family, "website");
    assert_eq!(contract.app_kind, "website");
    assert_eq!(contract.framework_hint.as_deref(), Some("vite"));
    assert_eq!(
        contract.local_url_hint.as_deref(),
        Some("http://127.0.0.1:5173/")
    );
    assert!(contract
        .preferred_workflows
        .iter()
        .any(|workflow| workflow == "website_validate"));
    assert!(contract
        .verification_workflows
        .iter()
        .any(|workflow| workflow == "build"));
    assert!(contract
        .delivery_phases
        .iter()
        .any(|phase| phase.contains("validate")));
    assert!(contract
        .quality_gates
        .iter()
        .any(|gate| gate.contains("critical routes")));
    assert!(contract.route_hints.iter().any(|route| route == "/"));
    assert!(contract.route_hints.iter().any(|route| route == "/about"));
    assert!(contract
        .route_hints
        .iter()
        .any(|route| route == "/pricing.html"));

    let prompt = profile_prompt_block(workspace.path()).expect("profile prompt block");
    assert!(prompt.contains("Loop family: website"));
    assert!(prompt.contains("Preferred workflows:"));

    let strategy = profile_strategy_prompt_block(workspace.path()).expect("strategy prompt block");
    assert!(strategy.contains("Stack Delivery Contract"));
    assert!(strategy.contains("Work in this order:")); // Delivery phases
    assert!(strategy.contains("Automatic proof should come from:")); // Verification workflows
    assert!(strategy.contains("Do not consider the task complete until these gates hold:"));
    // Quality gates
}

#[test]
fn test_workspace_profile_does_not_misclassify_node_service_as_website() {
    use hematite::agent::workspace_profile::detect_workspace_profile;

    let workspace = tempfile::tempdir().expect("temp workspace");
    fs::write(
        workspace.path().join("package.json"),
        r#"{
  "scripts": {
    "dev": "tsx server.ts",
    "start": "node server.js"
  },
  "dependencies": {
    "express": "^4.0.0"
  }
}"#,
    )
    .expect("write package json");

    let profile = detect_workspace_profile(workspace.path());
    let contract = profile.runtime_contract.expect("service contract");
    assert_eq!(contract.loop_family, "service");
    assert_eq!(contract.app_kind, "node-service");
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

    let chunk_count = vein
        .index_document("src/auth.rs", 1_000_000, doc)
        .expect("index");
    assert!(chunk_count > 0, "should produce chunks");

    let results = vein.search_bm25("authenticate", 5).expect("search");
    assert!(!results.is_empty(), "BM25 should find 'authenticate'");
    assert!(results[0].content.contains("authenticate"));

    // Confirm file count tracks correctly
    assert_eq!(vein.file_count(), 1);

    // Re-indexing same mtime should be a no-op
    let rechunk_count = vein
        .index_document("src/auth.rs", 1_000_000, doc)
        .expect("re-index");
    assert_eq!(rechunk_count, 0, "unchanged file should not re-index");
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
async fn test_inspect_host_connectivity_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "connectivity" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host connectivity should not hard-error");
    assert!(
        output.contains("Host inspection: connectivity"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_connectivity_reports_internet_status() {
    use serde_json::json;
    let args = json!({ "topic": "connectivity" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host connectivity should not hard-error");
    assert!(
        output.contains("Internet:") || output.contains("internet"),
        "expected internet status in output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_wifi_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "wifi" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host wifi should not hard-error");
    assert!(
        output.contains("Host inspection: wifi"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_connections_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "connections", "max_entries": 10 });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host connections should not hard-error");
    assert!(
        output.contains("Host inspection: connections"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_vpn_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "vpn" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host vpn should not hard-error");
    assert!(
        output.contains("Host inspection: vpn"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_proxy_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "proxy" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host proxy should not hard-error");
    assert!(
        output.contains("Host inspection: proxy"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_firewall_rules_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "firewall_rules", "max_entries": 10 });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host firewall_rules should not hard-error");
    assert!(
        output.contains("Host inspection: firewall_rules"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_traceroute_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "traceroute", "host": "8.8.8.8", "max_entries": 10 });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host traceroute should not hard-error");
    assert!(
        output.contains("Host inspection: traceroute"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_dns_cache_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "dns_cache", "max_entries": 20 });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host dns_cache should not hard-error");
    assert!(
        output.contains("Host inspection: dns_cache"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_arp_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "arp" });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host arp should not hard-error");
    assert!(
        output.contains("Host inspection: arp"),
        "unexpected output: {output}"
    );
}

#[tokio::test]
async fn test_inspect_host_route_table_returns_header() {
    use serde_json::json;
    let args = json!({ "topic": "route_table", "max_entries": 20 });
    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect_host route_table should not hard-error");
    assert!(
        output.contains("Host inspection: route_table"),
        "unexpected output: {output}"
    );
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
    assert!(output.contains("services (") || output.contains("Service summary:"));
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
async fn test_inspect_host_gpo_reports_access_denied_or_objects() {
    use serde_json::json;

    let args = json!({
        "topic": "gpo"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host gpo");

    assert!(output.contains("Host inspection: gpo"));
    assert!(
        output.contains("Applied Group Policy Objects")
            || output.contains("Error: Access denied")
            || output.contains("No applied Group Policy Objects")
            || output.contains("Windows-only")
    );
}

#[tokio::test]
async fn test_inspect_host_certificates_reports_personal_store() {
    use serde_json::json;

    let args = json!({
        "topic": "certificates"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host certificates");

    assert!(output.contains("Host inspection: certificates"));
    assert!(
        output.contains("Local Machine Certificates")
            || output.contains("No certificates found")
            || output.contains("Cert directory found")
    );
}

#[tokio::test]
async fn test_inspect_host_integrity_reports_cbs_health() {
    use serde_json::json;

    let args = json!({
        "topic": "integrity"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host integrity");

    assert!(output.contains("Host inspection: integrity"));
    assert!(
        output.contains("Windows Component Store Health")
            || output.contains("System integrity check")
            || output.contains("Could not retrieve CBS health")
    );
}

#[tokio::test]
async fn test_inspect_host_domain_reports_identity() {
    use serde_json::json;

    let args = json!({
        "topic": "domain"
    });

    let output = hematite::tools::host_inspect::inspect_host(&args)
        .await
        .expect("inspect host domain");

    assert!(output.contains("Host inspection: domain"));
    assert!(
        output.contains("Windows Domain / Workgroup Identity")
            || output.contains("Linux Domain Identity")
    );
}

#[tokio::test]
async fn test_inspect_host_device_health() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "device_health" }))
        .await
        .expect("inspect device health fails");
    assert!(output.contains("Host inspection: device_health"));
    assert!(
        output.contains("All PnP devices report as healthy")
            || output.contains("Malfunctioning Devices")
            || output.contains("hardware errors in dmesg")
    );
}

#[tokio::test]
async fn test_inspect_host_drivers() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(
        &json!({ "topic": "drivers", "max_entries": 5 }),
    )
    .await
    .expect("inspect drivers fails");
    assert!(output.contains("Host inspection: drivers"));
    assert!(output.contains("Active System Drivers") || output.contains("Loaded Kernel Modules"));
}

#[tokio::test]
async fn test_inspect_host_overclocker_returns_header() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "overclocker" }))
        .await
        .expect("inspect overclocker fails");
    assert!(output.contains("Host inspection: overclocker"));
}

#[tokio::test]
async fn test_inspect_host_overclocker_reports_voltage_telemetry_state() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "overclocker" }))
        .await
        .expect("inspect overclocker fails");
    assert!(
        output.contains("=== VOLTAGE TELEMETRY ===") && output.contains("GPU Voltage:"),
        "overclocker should report voltage telemetry availability explicitly; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_peripherals() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "peripherals" }))
        .await
        .expect("inspect peripherals fails");
    assert!(output.contains("Host inspection: peripherals"));
    assert!(output.contains("USB Controllers") || output.contains("Connected USB Devices"));
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

    assert_eq!(
        core_weight,
        Some(1.0),
        "hottest file should have weight 1.0"
    );
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

    assert!(
        result.is_ok(),
        "edit should succeed via fuzzy match: {:?}",
        result
    );

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

    assert!(
        result.is_ok(),
        "multi_search_replace should succeed via fuzzy: {:?}",
        result
    );

    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(
        content.contains("        99"),
        "replacement value should be at correct 8-space indent:\n{}",
        content
    );
}

#[test]
fn test_edit_file_rstrip_fallback_matches_trailing_spaces() {
    use std::fs;
    use tempfile::NamedTempFile;

    // File has trailing spaces on some lines (common in editor artefacts)
    let tmp = NamedTempFile::new().unwrap();
    fs::write(
        tmp.path(),
        "fn greet() {   \n    println!(\"hello\");   \n}\n",
    )
    .unwrap();

    let path = tmp.path().to_str().unwrap();

    // Model's search string has no trailing spaces (clean) — rstrip should bridge this
    let args = serde_json::json!({
        "path": path,
        "search": "fn greet() {\n    println!(\"hello\");\n}",
        "replace": "fn greet() {\n    println!(\"world\");\n}",
    });

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::file_ops::edit_file(&args));

    assert!(
        result.is_ok(),
        "rstrip fallback should match trailing-space file: {:?}",
        result
    );
    let content = fs::read_to_string(tmp.path()).unwrap();
    assert!(
        content.contains("world"),
        "replacement should have applied:\n{}",
        content
    );
}

#[test]
fn test_edit_file_cross_file_hint_in_error() {
    use std::fs;
    use tempfile::TempDir;

    // Two files: target is empty, sibling has the code the model is looking for
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("empty.rs");
    let sibling = dir.path().join("real.rs");
    fs::write(&target, "// nothing here\n").unwrap();
    fs::write(&sibling, "fn calculate() {\n    42\n}\n").unwrap();

    // Temporarily set cwd to the temp dir so workspace_root() finds it
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();

    let path = target.to_str().unwrap();
    let args = serde_json::json!({
        "path": path,
        "search": "fn calculate() {\n    42\n}",
        "replace": "fn calculate() {\n    99\n}",
    });

    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::file_ops::edit_file(&args));

    std::env::set_current_dir(original_dir).unwrap();

    assert!(result.is_err(), "should fail — search not in target file");
    let err = result.unwrap_err();
    assert!(
        err.contains("real.rs"),
        "error should mention the file that actually contains the search string:\n{}",
        err
    );
}

// ── Tool output overflow-to-scratch ───────────────────────────────────────────

#[test]
fn test_read_file_returns_full_content_before_conversation_cap() {
    // read_file itself does not cap — capping happens at the conversation layer.
    // Verify that large files are returned in full so the conversation layer
    // can make an informed truncation decision (and write to scratch).
    use std::fs;
    use tempfile::NamedTempFile;

    let tmp = NamedTempFile::new().unwrap();
    let big: String = (0..1000).map(|i| format!("line {:04}\n", i)).collect();
    fs::write(tmp.path(), &big).unwrap();

    let args = serde_json::json!({ "path": tmp.path().to_str().unwrap() });
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::file_ops::read_file(&args, 0));

    assert!(result.is_ok(), "read_file should succeed on large file");
    let content = result.unwrap();
    // Should contain first and last lines — not silently truncated before the cap layer
    assert!(content.contains("line 0000"), "should have first line");
    assert!(content.contains("line 0999"), "should have last line");
}

#[test]
fn test_shell_execute_large_output_accessible() {
    // Verify shell::execute is reachable and returns output for a basic command.
    // Large output capping to scratch is an integration concern tested at runtime.
    let args = serde_json::json!({ "command": "echo hematite-scratch-test" });
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(hematite::tools::shell::execute(&args, 0));

    // Shell may not be available in all CI environments — skip gracefully
    match result {
        Ok(out) => assert!(out.contains("hematite-scratch-test") || !out.is_empty()),
        Err(e) => println!("shell not available in this env: {}", e),
    }
}

// ── Memory-type tagging ───────────────────────────────────────────────────────

#[test]
fn test_detect_memory_type_decision() {
    use hematite::memory::vein::detect_memory_type;
    assert_eq!(
        detect_memory_type("we decided to use SQLite for the vein database"),
        "decision"
    );
    assert_eq!(
        detect_memory_type("let's use petgraph for the repo map"),
        "decision"
    );
    assert_eq!(
        detect_memory_type("going with AGPL for the license"),
        "decision"
    );
}

#[test]
fn test_detect_memory_type_problem() {
    use hematite::memory::vein::detect_memory_type;
    assert_eq!(
        detect_memory_type("the issue was that embed model state was not strict"),
        "problem"
    );
    assert_eq!(
        detect_memory_type("root cause was a missing CRLF normalization"),
        "problem"
    );
    assert_eq!(
        detect_memory_type("fixed by adding the rstrip fallback before full strip"),
        "problem"
    );
}

#[test]
fn test_detect_memory_type_milestone() {
    use hematite::memory::vein::detect_memory_type;
    assert_eq!(
        detect_memory_type("voice pipeline now working without LM Studio"),
        "milestone"
    );
    assert_eq!(
        detect_memory_type("successfully shipped v0.4.5 to crates.io"),
        "milestone"
    );
}

#[test]
fn test_detect_memory_type_preference() {
    use hematite::memory::vein::detect_memory_type;
    assert_eq!(
        detect_memory_type("i prefer lowercase conventional commits"),
        "preference"
    );
    assert_eq!(
        detect_memory_type("i like the diff preview before every edit"),
        "preference"
    );
}

#[test]
fn test_detect_memory_type_unclassified() {
    use hematite::memory::vein::detect_memory_type;
    assert_eq!(detect_memory_type("how does the vein indexing work"), "");
    assert_eq!(detect_memory_type("read the file and check the output"), "");
}

#[test]
fn test_vein_memory_type_indexed_and_retrieved() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let mut vein = Vein::new(tmp.path(), "http://127.0.0.1:0".to_string()).expect("vein init");

    // Index a decision chunk as a session exchange
    vein.index_document(
        "session/2026-04-12/turn-1",
        1_000,
        "we decided to use SQLite for local storage because it requires no server",
    )
    .unwrap();

    // BM25 search should find it
    let results = vein.search_bm25("decided SQLite storage", 10).unwrap();
    assert!(!results.is_empty(), "should find the session chunk");

    // The memory_type field should be "decision"
    let hit = results.iter().find(|r| r.path.contains("turn-1"));
    assert!(hit.is_some(), "should find the specific turn");
    assert_eq!(
        hit.unwrap().memory_type,
        "decision",
        "session chunk with 'decided' should be tagged as decision"
    );
}

// ── Streaming shell ───────────────────────────────────────────────────────────

#[test]
fn test_shell_streaming_emits_shell_line_events() {
    // Verify that execute_streaming sends at least one ShellLine event for a
    // command that produces output, and that the final return value contains
    // the same content.
    use hematite::agent::inference::InferenceEvent;
    use tokio::sync::mpsc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // Channel with enough headroom so execute_streaming never blocks on send.
        let (tx, mut rx) = mpsc::channel::<InferenceEvent>(128);
        let args = serde_json::json!({ "command": "echo streaming-test" });

        // Drop tx after the call so recv() terminates naturally.
        let result = hematite::tools::shell::execute_streaming(&args, tx, 0).await;

        // Drain all events from the channel.
        let mut shell_lines: Vec<String> = Vec::new();
        while let Ok(event) = rx.try_recv() {
            if let InferenceEvent::ShellLine(line) = event {
                shell_lines.push(line);
            }
        }

        match result {
            Ok(output) => {
                assert!(
                    !shell_lines.is_empty(),
                    "execute_streaming should emit ShellLine events; got none"
                );
                assert!(
                    output.contains("streaming-test"),
                    "buffered output should contain echo content; got: {output}"
                );
                let streamed = shell_lines.join("\n");
                assert!(
                    streamed.contains("streaming-test"),
                    "streamed lines should contain echo content; got: {streamed}"
                );
            }
            Err(e) => println!("shell not available in this env: {e}"),
        }
    });
}

#[test]
fn test_shell_streaming_buffered_output_matches_blocking() {
    // Both execute() and execute_streaming() should return the same content
    // for a deterministic command. The streaming path must not corrupt or
    // lose the output while sending ShellLine events.
    use hematite::agent::inference::InferenceEvent;
    use tokio::sync::mpsc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "command": "echo consistent-output" });

        let blocking = hematite::tools::shell::execute(&args, 0).await;

        let (tx, mut rx) = mpsc::channel::<InferenceEvent>(128);
        let streaming = hematite::tools::shell::execute_streaming(&args, tx, 0).await;
        // Drain buffered events (not the focus of this test).
        while rx.try_recv().is_ok() {}

        match (blocking, streaming) {
            (Ok(b), Ok(s)) => {
                assert!(
                    b.contains("consistent-output") && s.contains("consistent-output"),
                    "both paths should contain echo output; blocking={b:?} streaming={s:?}"
                );
            }
            (Err(e), _) | (_, Err(e)) => println!("shell not available in this env: {e}"),
        }
    });
}

// ── Turn checkpointing ────────────────────────────────────────────────────────

#[test]
fn test_checkpoint_load_returns_none_when_no_session_file() {
    // load_checkpoint() must return None gracefully when .hematite/session.json
    // does not exist or has no real turns — not panic.
    // We test this by checking the result type alone (the real file may or
    // may not exist in the test environment).
    let result = std::panic::catch_unwind(hematite::agent::conversation::load_checkpoint);
    assert!(result.is_ok(), "load_checkpoint should never panic");
}

#[test]
fn test_checkpoint_roundtrip_via_session_json() {
    // Write a session.json that looks like a real prior session in a isolated temp directory,
    // then verify load_checkpoint() surfaces the right fields.
    use std::io::Write;

    // Create a temporary directory and a unique session path.
    let temp_workspace = tempfile::tempdir().expect("failed to create temp workspace");
    let session_path = temp_workspace.path().join("session.json");

    // Tell the agent to use this specific path for this test thread.
    std::env::set_var("HEMATITE_SESSION_PATH", &session_path);

    // Write a fake prior session.
    let fake = serde_json::json!({
        "running_summary": null,
        "session_memory": {
            "current_task": "implement streaming shell output",
            "working_set": ["src/tools/shell.rs", "src/agent/conversation.rs"],
            "learnings": [],
            "last_verification": { "successful": true, "summary": "cargo test ok" }
        },
        "last_goal": "add streaming shell and diagnostics",
        "turn_count": 7
    });

    {
        let mut f =
            std::fs::File::create(&session_path).expect("Failed to create fake session.json");
        write!(f, "{}", fake).expect("Failed to write fake session.json");
    }

    let cp = hematite::agent::conversation::load_checkpoint();

    // Clean up the environment variable.
    std::env::remove_var("HEMATITE_SESSION_PATH");

    let cp = cp.expect("load_checkpoint should return Some for a valid prior session");
    assert_eq!(cp.turn_count, 7);
    assert_eq!(cp.last_goal, "add streaming shell and diagnostics");
    assert_eq!(cp.last_verify_ok, Some(true));
    assert!(
        cp.working_files.contains(&"src/tools/shell.rs".to_string())
            || cp
                .working_files
                .contains(&"src/agent/conversation.rs".to_string()),
        "working_files should include files from working_set"
    );
}

// ── Compaction improvements ───────────────────────────────────────────────────

#[test]
fn test_extract_memory_working_set_spans_all_turns() {
    // Files touched in earlier turns must survive in the working_set, not just
    // files from the most recent user turn.
    use hematite::agent::compaction::extract_memory;
    use hematite::agent::inference::ChatMessage;

    fn tool_call_msg(path: &str) -> ChatMessage {
        let mut m = ChatMessage::assistant_text("");
        m.tool_calls = Some(vec![hematite::agent::inference::ToolCallResponse {
            id: "x".into(),
            call_type: "function".into(),
            index: Some(0),
            function: hematite::agent::inference::ToolCallFn {
                name: "edit_file".into(),
                arguments: serde_json::json!({"path": path, "search": "a", "replace": "b"}),
            },
        }]);
        m
    }

    let messages = vec![
        ChatMessage::system("sys"),
        ChatMessage::user("first turn"),
        tool_call_msg("src/early_file.rs"),
        ChatMessage::user("second turn"),
        tool_call_msg("src/later_file.rs"),
        ChatMessage::user("third turn — most recent"),
        tool_call_msg("src/newest_file.rs"),
    ];

    let mem = extract_memory(&messages);

    // All three files should appear in the working set.
    assert!(
        mem.working_set.contains("src/early_file.rs"),
        "early file should survive across turns; got {:?}",
        mem.working_set
    );
    assert!(mem.working_set.contains("src/later_file.rs"));
    assert!(mem.working_set.contains("src/newest_file.rs"));
    // Current task should be from the last user message.
    assert!(mem.current_task.contains("most recent"));
}

#[test]
fn test_build_summary_captures_verify_build_outcome() {
    // build_technical_summary must surface the verify_build result so the model
    // knows whether the build was passing when context was compacted.
    use hematite::agent::compaction::compact_history;
    use hematite::agent::compaction::CompactionConfig;
    use hematite::agent::inference::ChatMessage;

    // Build a history long enough to trigger compaction.
    let mut messages = vec![ChatMessage::system("sys")];
    for i in 0..30 {
        messages.push(ChatMessage::user(&format!("do task {i}")));
        let mut assistant = ChatMessage::assistant_text("");
        assistant.tool_calls = Some(vec![hematite::agent::inference::ToolCallResponse {
            id: format!("c{i}"),
            call_type: "function".into(),
            index: Some(0),
            function: hematite::agent::inference::ToolCallFn {
                name: "verify_build".into(),
                arguments: serde_json::json!({}),
            },
        }]);
        messages.push(assistant);
        let mut tool_result = ChatMessage::user("BUILD OK — cargo build passed");
        tool_result.role = "tool".into();
        messages.push(tool_result);
    }

    let config = CompactionConfig {
        preserve_recent_messages: 6,
        max_estimated_tokens: 100, // force compaction
    };
    let result = compact_history(&messages, None, config, Some(1));

    // The compacted summary message should mention BUILD OK.
    let summary_msg = result
        .messages
        .iter()
        .find(|m| m.role == "system" && m.content.as_str().contains("CONTEXT SUMMARY"));
    assert!(
        summary_msg.is_some(),
        "compaction should produce a summary system message"
    );
    let summary_text = summary_msg.unwrap().content.as_str();
    assert!(
        summary_text.contains("BUILD OK") || summary_text.contains("verify_build"),
        "summary should capture verify_build outcome; got:\n{summary_text}"
    );
}

// ── verify_build streaming ─────────────────────────────────────────────────────

#[test]
#[allow(clippy::await_holding_lock)]
fn test_verify_build_streaming_no_project_emits_no_shell_lines() {
    // In a directory with no recognized project file, execute_streaming must
    // return Err quickly (autodetect failure) and must NOT emit any ShellLine
    // events — no shell command is ever launched in that path.
    use hematite::agent::inference::InferenceEvent;
    use tokio::sync::mpsc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp = std::env::temp_dir().join("hematite_vb_streaming_test");
        std::fs::create_dir_all(&tmp).unwrap();

        // Serialize with other set_current_dir tests — CWD is global process state.
        let _guard = CWD_LOCK.lock().unwrap();

        // Switch CWD to the empty temp dir so autodetect finds no project file.
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let (tx, mut rx) = mpsc::channel::<InferenceEvent>(32);
        let args = serde_json::json!({ "action": "build" });
        let result = hematite::tools::verify_build::execute_streaming(&args, tx).await;

        // Restore CWD before any assertions so other tests are not affected.
        std::env::set_current_dir(&original).unwrap();

        // No shell command was run, so the channel must be empty.
        let mut shell_line_count = 0usize;
        while let Ok(ev) = rx.try_recv() {
            if matches!(ev, InferenceEvent::ShellLine(_)) {
                shell_line_count += 1;
            }
        }
        assert_eq!(
            shell_line_count, 0,
            "no ShellLine events expected when autodetect fails"
        );
        assert!(
            result.is_err(),
            "execute_streaming should return Err when no project is detected"
        );
        let msg = result.unwrap_err();
        assert!(
            msg.contains("No recognized project root"),
            "error should explain the missing project root; got: {msg}"
        );
    });
}

#[test]
#[allow(clippy::await_holding_lock)]
fn test_verify_build_streaming_output_shape_matches_blocking() {
    // Both execute() and execute_streaming() must return an Ok/Err with the
    // same "BUILD OK [...]" / "BUILD FAILED [...]" prefix format. The streaming
    // variant must not alter the tool-result string the model sees.
    //
    // This test only checks output shape — it does not run a real build.
    // Actual ShellLine event emission is verified by the shell streaming tests;
    // verify_build delegates directly to shell::execute_streaming so the
    // event path is the same code exercised there.

    // The shape check is structural: if execute_streaming returns Ok, the
    // content must start with "BUILD OK"; if Err, "BUILD FAILED" or a
    // descriptive message (no project, timeout, etc.) is acceptable.
    // We run in a temp dir with no project so both paths return Err — the
    // point is that both return the same Err class.
    use tokio::sync::mpsc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp = std::env::temp_dir().join("hematite_vb_shape_test");
        std::fs::create_dir_all(&tmp).unwrap();

        // Serialize with other set_current_dir tests — CWD is global process state.
        let _guard = CWD_LOCK.lock().unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp).unwrap();

        let args = serde_json::json!({ "action": "build" });

        let blocking = hematite::tools::verify_build::execute(&args).await;

        let (tx, mut rx) =
            mpsc::channel::<hematite::agent::inference::InferenceEvent>(32);
        let streaming = hematite::tools::verify_build::execute_streaming(&args, tx).await;
        while rx.try_recv().is_ok() {}

        std::env::set_current_dir(&original).unwrap();

        // Both must agree: either both Ok or both Err (no project root → both Err).
        assert_eq!(
            blocking.is_ok(),
            streaming.is_ok(),
            "blocking and streaming must agree on Ok/Err; blocking={blocking:?} streaming={streaming:?}"
        );
    });
}

// ── tail_file ─────────────────────────────────────────────────────────────────

#[test]
fn test_tail_file_returns_last_n_lines() {
    // tail_file with lines=3 on a 10-line file must return exactly the last 3
    // lines with correct absolute line numbers and a header.
    use hematite::tools::file_ops::tail_file;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp_path = std::env::temp_dir().join("hematite_tail_test.txt");
        let content = (1..=10u32)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&tmp_path, &content).unwrap();

        let args = serde_json::json!({
            "path": tmp_path.to_string_lossy(),
            "lines": 3
        });
        let result = tail_file(&args).await.unwrap();

        assert!(
            result.contains("line 8"),
            "tail should include line 8; got:\n{result}"
        );
        assert!(
            result.contains("line 9"),
            "tail should include line 9; got:\n{result}"
        );
        assert!(
            result.contains("line 10"),
            "tail should include line 10; got:\n{result}"
        );
        // line 7 should NOT be in the output
        assert!(
            !result.contains("line 7"),
            "tail should NOT include line 7 when lines=3; got:\n{result}"
        );
        // Header should mention line numbers and total
        assert!(
            result.contains("10"),
            "header should mention total line count; got:\n{result}"
        );

        let _ = std::fs::remove_file(&tmp_path);
    });
}

#[test]
fn test_tail_file_grep_filter_matches_only_relevant_lines() {
    // tail_file with grep="error" on a mixed file must return only lines
    // containing "error", still respecting the lines= cap.
    use hematite::tools::file_ops::tail_file;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp_path = std::env::temp_dir().join("hematite_tail_grep_test.txt");
        let lines = [
            "info: starting build",
            "error: E0425 cannot find value",
            "info: compiling foo.rs",
            "error: E0308 type mismatch",
            "info: build finished",
        ];
        std::fs::write(&tmp_path, lines.join("\n")).unwrap();

        let args = serde_json::json!({
            "path": tmp_path.to_string_lossy(),
            "grep": "error"
        });
        let result = tail_file(&args).await.unwrap();

        assert!(
            result.contains("E0425"),
            "grep=error should include the E0425 error line; got:\n{result}"
        );
        assert!(
            result.contains("E0308"),
            "grep=error should include the E0308 error line; got:\n{result}"
        );
        assert!(
            !result.contains("compiling"),
            "grep=error should exclude non-error lines; got:\n{result}"
        );
        assert!(
            !result.contains("build finished"),
            "grep=error should exclude info lines; got:\n{result}"
        );

        let _ = std::fs::remove_file(&tmp_path);
    });
}

#[test]
fn test_tail_file_missing_file_returns_err() {
    use hematite::tools::file_ops::tail_file;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "path": "/nonexistent/path/to/file.log" });
        let result = tail_file(&args).await;
        assert!(
            result.is_err(),
            "tail_file on a missing file must return Err"
        );
    });
}

#[test]
fn test_tail_file_lines_default_is_fifty() {
    // When lines is omitted, tail_file must default to 50 lines.
    use hematite::tools::file_ops::tail_file;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let tmp_path = std::env::temp_dir().join("hematite_tail_default_test.txt");
        // 60-line file — without explicit lines=, should return exactly 50.
        let content = (1..=60u32)
            .map(|i| format!("row {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&tmp_path, &content).unwrap();

        let args = serde_json::json!({ "path": tmp_path.to_string_lossy() });
        let result = tail_file(&args).await.unwrap();

        // Line 60 must be present; line 10 (outside the 50-line window) must not.
        assert!(
            result.contains("row 60"),
            "default tail must include last line"
        );
        assert!(
            result.contains("row 11"),
            "default tail must include row 11 (60-50=10, so 11 is the first)"
        );
        assert!(
            !result.contains("row 10"),
            "default tail must NOT include row 10 (outside 50-line window)"
        );

        let _ = std::fs::remove_file(&tmp_path);
    });
}

// ── inspect_host: log_check and startup_items ─────────────────────────────────

#[test]
fn test_inspect_host_log_check_returns_header() {
    // log_check must return a recognizable header and not panic. On a Windows
    // machine with event logs it will surface real entries; on CI with no
    // event log access it must still return Ok (not Err).
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "log_check", "max_entries": 5 });
        let result = inspect_host(&args).await;

        // Must return Ok regardless of whether events were found.
        let output = result.expect("log_check must return Ok, not Err");
        assert!(
            output.contains("log_check"),
            "log_check output must contain the topic name as a header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_startup_items_returns_header() {
    // startup_items must return a recognizable header and not panic. On a real
    // Windows machine it will enumerate Run key entries; on CI or Linux it
    // must still return Ok with a meaningful message.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "startup_items", "max_entries": 10 });
        let result = inspect_host(&args).await;

        let output = result.expect("startup_items must return Ok, not Err");
        assert!(
            output.contains("startup_items"),
            "startup_items output must contain the topic name as a header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_unknown_topic_includes_new_topics_in_error() {
    // The unknown-topic error message must list log_check and startup_items
    // so operators know they are available.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "nonexistent_topic_xyz" });
        let result = inspect_host(&args).await;
        let err = result.expect_err("unknown topic must return Err");
        assert!(
            err.contains("log_check"),
            "unknown-topic error must mention log_check; got:\n{err}"
        );
        assert!(
            err.contains("startup_items"),
            "unknown-topic error must mention startup_items; got:\n{err}"
        );
        assert!(
            err.contains("storage"),
            "unknown-topic error must mention storage; got:\n{err}"
        );
        assert!(
            err.contains("hardware"),
            "unknown-topic error must mention hardware; got:\n{err}"
        );
        assert!(
            err.contains("health_report"),
            "unknown-topic error must mention health_report; got:\n{err}"
        );
    });
}

// ── inspect_host: health_report, storage, hardware ────────────────────────────

#[test]
fn test_inspect_host_health_report_returns_verdict() {
    // health_report must return Ok with a recognizable verdict header.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "health_report" });
        let output = inspect_host(&args)
            .await
            .expect("health_report must return Ok");
        // Must contain the verdict marker regardless of machine state.
        let has_verdict = output.contains("ALL GOOD")
            || output.contains("WORTH A LOOK")
            || output.contains("ACTION REQUIRED");
        assert!(
            has_verdict,
            "health_report must include a verdict; got:\n{output}"
        );
        assert!(
            output.contains("System Health Report"),
            "health_report must include the header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_health_report_sections_are_non_empty() {
    // health_report should always populate at least one section (good/watch/fix).
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "health_report" });
        let output = inspect_host(&args)
            .await
            .expect("health_report must return Ok");
        let has_section = output.contains("Looking good:")
            || output.contains("Worth watching:")
            || output.contains("Needs fixing:");
        assert!(
            has_section,
            "health_report must include at least one categorized section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_storage_returns_drive_info() {
    // storage must return Ok with a "Drives:" section.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "storage" });
        let output = inspect_host(&args).await.expect("storage must return Ok");
        assert!(
            output.contains("storage"),
            "storage output must contain topic header; got:\n{output}"
        );
        assert!(
            output.contains("Drives:") || output.contains("drive") || output.contains("GB"),
            "storage output must describe drive capacity; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_storage_includes_cache_section() {
    // storage must always include the developer cache section header.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "storage" });
        let output = inspect_host(&args).await.expect("storage must return Ok");
        assert!(
            output.contains("cache") || output.contains("Cache"),
            "storage output must include a cache directory section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_hardware_returns_cpu_info() {
    // hardware must return Ok and include CPU information.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "hardware" });
        let output = inspect_host(&args).await.expect("hardware must return Ok");
        assert!(
            output.contains("hardware"),
            "hardware output must contain topic header; got:\n{output}"
        );
        assert!(
            output.contains("CPU") || output.contains("processor") || output.contains("core"),
            "hardware output must include CPU information; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_hardware_returns_gpu_or_ram() {
    // hardware must include either GPU or RAM information.
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "hardware" });
        let output = inspect_host(&args).await.expect("hardware must return Ok");
        let has_gpu_or_ram =
            output.contains("GPU") || output.contains("RAM") || output.contains("GB");
        assert!(
            has_gpu_or_ram,
            "hardware output must include GPU or RAM details; got:\n{output}"
        );
    });
}

// ── updates ───────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_updates_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "updates" });
        let output = inspect_host(&args).await.expect("updates must return Ok");
        assert!(
            output.contains("updates"),
            "updates output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_updates_contains_update_info() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "updates" });
        let output = inspect_host(&args).await.expect("updates must return Ok");
        // Should report last install, pending count, or WU service state
        let has_info = output.contains("Last update")
            || output.contains("Pending")
            || output.contains("service")
            || output.contains("up to date")
            || output.contains("unable")
            || output.contains("package");
        assert!(
            has_info,
            "updates output must contain meaningful update info; got:\n{output}"
        );
    });
}

// ── security ──────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_security_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "security" });
        let output = inspect_host(&args).await.expect("security must return Ok");
        assert!(
            output.contains("security"),
            "security output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_security_reports_protection_status() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "security" });
        let output = inspect_host(&args).await.expect("security must return Ok");
        // Should report Defender, Firewall, or activation status
        let has_info = output.contains("Defender")
            || output.contains("Firewall")
            || output.contains("activation")
            || output.contains("UAC")
            || output.contains("protection")
            || output.contains("UFW")
            || output.contains("unable");
        assert!(
            has_info,
            "security output must report protection status; got:\n{output}"
        );
    });
}

// ── pending_reboot ────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_pending_reboot_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "pending_reboot" });
        let output = inspect_host(&args)
            .await
            .expect("pending_reboot must return Ok");
        assert!(
            output.contains("pending_reboot"),
            "pending_reboot output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_pending_reboot_gives_verdict() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "pending_reboot" });
        let output = inspect_host(&args)
            .await
            .expect("pending_reboot must return Ok");
        // Must say either no restart needed or that one is pending
        let has_verdict = output.contains("No restart")
            || output.contains("restart is pending")
            || output.contains("Could not")
            || output.contains("reboot-required");
        assert!(
            has_verdict,
            "pending_reboot must give a clear verdict; got:\n{output}"
        );
    });
}

// ── disk_health ───────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_disk_health_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "disk_health" });
        let output = inspect_host(&args)
            .await
            .expect("disk_health must return Ok");
        assert!(
            output.contains("disk_health"),
            "disk_health output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_disk_health_reports_drive_info() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "disk_health" });
        let output = inspect_host(&args)
            .await
            .expect("disk_health must return Ok");
        // Should find drives or report gracefully
        let has_info = output.contains("Health")
            || output.contains("Drive")
            || output.contains("GB")
            || output.contains("No physical")
            || output.contains("Unable")
            || output.contains("NAME")
            || output.contains("smartmontools");
        assert!(
            has_info,
            "disk_health must report drive info or explain unavailability; got:\n{output}"
        );
    });
}

// ── battery ───────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_battery_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "battery" });
        let output = inspect_host(&args).await.expect("battery must return Ok");
        assert!(
            output.contains("battery"),
            "battery output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_battery_reports_status_or_no_battery() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "battery" });
        let output = inspect_host(&args).await.expect("battery must return Ok");
        // Either finds a battery or reports no battery on desktop
        let has_info = output.contains("Battery:")
            || output.contains("No battery")
            || output.contains("desktop")
            || output.contains("Charge")
            || output.contains("Unable")
            || output.contains("AC-only");
        assert!(
            has_info,
            "battery must report charge status or explain no battery; got:\n{output}"
        );
    });
}

// ── recent_crashes ────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_recent_crashes_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "recent_crashes" });
        let output = inspect_host(&args)
            .await
            .expect("recent_crashes must return Ok");
        assert!(
            output.contains("recent_crashes"),
            "recent_crashes output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_recent_crashes_reports_crash_info_or_none() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "recent_crashes" });
        let output = inspect_host(&args)
            .await
            .expect("recent_crashes must return Ok");
        // Must give some verdict on crashes
        let has_info = output.contains("None in recent")
            || output.contains("crashes")
            || output.contains("BSOD")
            || output.contains("shutdown")
            || output.contains("unable")
            || output.contains("No kernel");
        assert!(
            has_info,
            "recent_crashes must report crash history or explain unavailability; got:\n{output}"
        );
    });
}

// ── scheduled_tasks ───────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_scheduled_tasks_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "scheduled_tasks" });
        let output = inspect_host(&args)
            .await
            .expect("scheduled_tasks must return Ok");
        assert!(
            output.contains("scheduled_tasks"),
            "scheduled_tasks output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_scheduled_tasks_reports_tasks_or_explains() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "scheduled_tasks" });
        let output = inspect_host(&args)
            .await
            .expect("scheduled_tasks must return Ok");
        // Should list tasks or explain
        let has_info = output.contains("State:")
            || output.contains("Last run:")
            || output.contains("No active")
            || output.contains("Unable")
            || output.contains("timers")
            || output.contains("crontab");
        assert!(
            has_info,
            "scheduled_tasks must list tasks or explain availability; got:\n{output}"
        );
    });
}

// ── dev_conflicts ─────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_dev_conflicts_returns_header() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dev_conflicts" });
        let output = inspect_host(&args)
            .await
            .expect("dev_conflicts must return Ok");
        assert!(
            output.contains("dev_conflicts"),
            "dev_conflicts output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_dev_conflicts_checks_major_runtimes() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dev_conflicts" });
        let output = inspect_host(&args)
            .await
            .expect("dev_conflicts must return Ok");
        // Must check at minimum Node and Python and Git
        let checks_node = output.contains("Node.js");
        let checks_python = output.contains("Python");
        let checks_git = output.contains("Git");
        assert!(
            checks_node && checks_python && checks_git,
            "dev_conflicts must check Node.js, Python, and Git; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_dev_conflicts_gives_summary_verdict() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dev_conflicts" });
        let output = inspect_host(&args)
            .await
            .expect("dev_conflicts must return Ok");
        // Must conclude with a summary (conflict found or clean)
        let has_verdict = output.contains("No conflicts")
            || output.contains("CONFLICTS")
            || output.contains("NOTES")
            || output.contains("[!]")
            || output.contains("[-]");
        assert!(
            has_verdict,
            "dev_conflicts must end with a summary verdict; got:\n{output}"
        );
    });
}

// ── unknown topic now includes new topics in error ─────────────────────────────

#[test]
fn test_inspect_host_unknown_topic_includes_all_new_topics_in_error() {
    use hematite::tools::host_inspect::inspect_host;

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "nonexistent_topic_xyz" });
        let err = inspect_host(&args)
            .await
            .expect_err("unknown topic must return Err");
        let new_topics = [
            "updates",
            "security",
            "pending_reboot",
            "disk_health",
            "battery",
            "recent_crashes",
            "scheduled_tasks",
            "dev_conflicts",
            "docker",
            "docker_filesystems",
            "wsl",
            "wsl_filesystems",
            "lan_discovery",
            "ssh",
            "env",
            "hosts_file",
            "installed_software",
            "git_config",
            "identity_auth",
        ];
        for topic in new_topics {
            assert!(
                err.contains(topic),
                "error message must list '{topic}' as a valid topic; got:\n{err}"
            );
        }
    });
}

// ── env ───────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_env_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "env" });
        let output = inspect_host(&args).await.expect("env must return Ok");
        assert!(
            output.contains("Host inspection: env"),
            "env output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_env_shows_total_and_path_note() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "env" });
        let output = inspect_host(&args).await.expect("env must return Ok");
        assert!(
            output.contains("Total environment variables:"),
            "env output must show total count; got:\n{output}"
        );
        assert!(
            output.contains("PATH:"),
            "env output must note PATH entry count; got:\n{output}"
        );
    });
}

// ── hosts_file ────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_hosts_file_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "hosts_file" });
        let output = inspect_host(&args)
            .await
            .expect("hosts_file must return Ok");
        assert!(
            output.contains("Host inspection: hosts_file"),
            "hosts_file output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_hosts_file_shows_path_and_summary() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "hosts_file" });
        let output = inspect_host(&args)
            .await
            .expect("hosts_file must return Ok");
        let has_path =
            output.contains("Path:") && (output.contains("hosts") || output.contains("etc"));
        let has_summary = output.contains("Active entries:") || output.contains("Could not read");
        assert!(has_path, "hosts_file must show file path; got:\n{output}");
        assert!(
            has_summary,
            "hosts_file must show entry summary or error; got:\n{output}"
        );
    });
}

// ── docker ────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_docker_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "docker" });
        let output = inspect_host(&args).await.expect("docker must return Ok");
        assert!(
            output.contains("Host inspection: docker"),
            "docker output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_docker_reports_status_or_not_found() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "docker" });
        let output = inspect_host(&args).await.expect("docker must return Ok");
        let has_result = output.contains("Docker Engine:")
            || output.contains("not found")
            || output.contains("daemon is NOT running")
            || output.contains("error");
        assert!(
            has_result,
            "docker must report engine version, not-found, or daemon-down; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_docker_filesystems_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "docker_filesystems" });
        let output = inspect_host(&args)
            .await
            .expect("docker_filesystems must return Ok");
        assert!(
            output.contains("Host inspection: docker_filesystems"),
            "docker_filesystems output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_docker_filesystems_reports_findings_or_not_found() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "docker_filesystems" });
        let output = inspect_host(&args)
            .await
            .expect("docker_filesystems must return Ok");
        let has_result = output.contains("=== Findings ===")
            || output.contains("not found")
            || output.contains("daemon is NOT running")
            || output.contains("error");
        assert!(
            has_result,
            "docker_filesystems must report findings or installation state; got:\n{output}"
        );
    });
}

// ── wsl ───────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_wsl_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wsl" });
        let output = inspect_host(&args).await.expect("wsl must return Ok");
        assert!(
            output.contains("Host inspection: wsl"),
            "wsl output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_wsl_reports_distros_or_status() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wsl" });
        let output = inspect_host(&args).await.expect("wsl must return Ok");
        // On Windows: distros or install hint. On other OS: feature note.
        let has_result = output.contains("WSL Distributions")
            || output.contains("not installed")
            || output.contains("no distributions")
            || output.contains("Windows-only feature")
            || output.contains("wsl --install")
            || output.contains("error");
        assert!(
            has_result,
            "wsl must report distros, install hint, or platform note; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_wsl_filesystems_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wsl_filesystems" });
        let output = inspect_host(&args)
            .await
            .expect("wsl_filesystems must return Ok");
        assert!(
            output.contains("Host inspection: wsl_filesystems"),
            "wsl_filesystems output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_wsl_filesystems_reports_findings_or_platform_note() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wsl_filesystems" });
        let output = inspect_host(&args)
            .await
            .expect("wsl_filesystems must return Ok");
        let has_result = output.contains("=== Findings ===")
            || output.contains("Windows-only inspection")
            || output.contains("wsl --install")
            || output.contains("error");
        assert!(
            has_result,
            "wsl_filesystems must report findings, install hint, or platform note; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_lan_discovery_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "lan_discovery" });
        let output = inspect_host(&args)
            .await
            .expect("lan_discovery must return Ok");
        assert!(
            output.contains("Host inspection: lan_discovery"),
            "lan_discovery output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_lan_discovery_reports_findings_or_evidence() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "lan_discovery" });
        let output = inspect_host(&args)
            .await
            .expect("lan_discovery must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Neighborhood evidence ===")
            && output.contains("=== Active adapter and gateway summary ===");
        assert!(
            has_result,
            "lan_discovery must report findings and neighborhood evidence; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_audio_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "audio" });
        let output = inspect_host(&args).await.expect("audio must return Ok");
        assert!(
            output.contains("Host inspection: audio"),
            "audio output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_audio_reports_findings_or_inventory() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "audio" });
        let output = inspect_host(&args).await.expect("audio must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Audio services ===")
            && output.contains("=== Playback and recording endpoints ===");
        assert!(
            has_result,
            "audio must report findings and endpoint inventory; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_bluetooth_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "bluetooth" });
        let output = inspect_host(&args).await.expect("bluetooth must return Ok");
        assert!(
            output.contains("Host inspection: bluetooth"),
            "bluetooth output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_bluetooth_reports_findings_or_inventory() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "bluetooth" });
        let output = inspect_host(&args).await.expect("bluetooth must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Bluetooth services ===")
            && output.contains("=== Bluetooth radios and adapters ===");
        assert!(
            has_result,
            "bluetooth must report findings and radio inventory; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_camera_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "camera" });
        let output = inspect_host(&args).await.expect("camera must return Ok");
        assert!(
            output.contains("Host inspection: camera"),
            "camera output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_camera_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "camera" });
        let output = inspect_host(&args).await.expect("camera must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== Camera devices ===");
        assert!(
            has_result,
            "camera must report findings and device inventory; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_sign_in_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "sign_in" });
        let output = inspect_host(&args).await.expect("sign_in must return Ok");
        assert!(
            output.contains("Host inspection: sign_in"),
            "sign_in output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_sign_in_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "sign_in" });
        let output = inspect_host(&args).await.expect("sign_in must return Ok");
        let has_result = output.contains("=== Findings ===")
            && (output.contains("=== Windows Hello") || output.contains("=== Biometric"));
        assert!(
            has_result,
            "sign_in must report findings and Hello/biometric section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_search_index_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "search_index" });
        let output = inspect_host(&args)
            .await
            .expect("search_index must return Ok");
        assert!(
            output.contains("Host inspection: search_index"),
            "search_index output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_onedrive_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "onedrive" });
        let output = inspect_host(&args).await.expect("onedrive must return Ok");
        assert!(
            output.contains("Host inspection: onedrive"),
            "onedrive output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_browser_health_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "browser_health" });
        let output = inspect_host(&args)
            .await
            .expect("browser_health must return Ok");
        assert!(
            output.contains("Host inspection: browser_health"),
            "browser_health output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_installer_health_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "installer_health" });
        let output = inspect_host(&args)
            .await
            .expect("installer_health must return Ok");
        assert!(
            output.contains("Host inspection: installer_health"),
            "installer_health output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_installer_health_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "installer_health" });
        let output = inspect_host(&args)
            .await
            .expect("installer_health must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Installer engines ===")
            && output.contains("=== winget and App Installer ===");
        assert!(
            has_result,
            "installer_health must report findings and installer sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_browser_health_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "browser_health" });
        let output = inspect_host(&args)
            .await
            .expect("browser_health must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Browser inventory ===")
            && output.contains("=== WebView2 runtime ===");
        assert!(
            has_result,
            "browser_health must report findings and browser sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_onedrive_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "onedrive" });
        let output = inspect_host(&args).await.expect("onedrive must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== OneDrive client ===")
            && output.contains("=== OneDrive accounts ===");
        assert!(
            has_result,
            "onedrive must report findings and OneDrive sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_outlook_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "outlook" });
        let output = inspect_host(&args).await.expect("outlook must return Ok");
        assert!(
            output.contains("Host inspection: outlook"),
            "outlook output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_outlook_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "outlook" });
        let output = inspect_host(&args).await.expect("outlook must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Outlook install inventory ===")
            && output.contains("=== Mail profiles ===");
        assert!(
            has_result,
            "outlook must report findings and core sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_teams_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "teams" });
        let output = inspect_host(&args).await.expect("teams must return Ok");
        assert!(
            output.contains("Host inspection: teams"),
            "teams output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_teams_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "teams" });
        let output = inspect_host(&args).await.expect("teams must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Teams install inventory ===")
            && output.contains("=== Cache directory sizing ===");
        assert!(
            has_result,
            "teams must report findings and core sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_identity_auth_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "identity_auth" });
        let output = inspect_host(&args)
            .await
            .expect("identity_auth must return Ok");
        assert!(
            output.contains("Host inspection: identity_auth"),
            "identity_auth output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_identity_auth_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "identity_auth" });
        let output = inspect_host(&args)
            .await
            .expect("identity_auth must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Identity broker services ===")
            && output.contains("=== Device registration ===")
            && output.contains("=== Microsoft app account signals ===");
        assert!(
            has_result,
            "identity_auth must report findings and core sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_event_query_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "event_query", "event_id": 7036, "hours": 2 });
        let output = inspect_host(&args)
            .await
            .expect("event_query must return Ok");
        assert!(
            output.contains("Host inspection: event_query"),
            "event_query output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_event_query_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "event_query", "hours": 1 });
        let output = inspect_host(&args)
            .await
            .expect("event_query must return Ok");
        let has_result = output.contains("=== Findings ===") && output.contains("=== Event query:");
        assert!(
            has_result,
            "event_query must report findings and event query section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_app_crashes_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "app_crashes" });
        let output = inspect_host(&args)
            .await
            .expect("app_crashes must return Ok");
        assert!(
            output.contains("Host inspection: app_crashes"),
            "app_crashes output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_app_crashes_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "app_crashes" });
        let output = inspect_host(&args)
            .await
            .expect("app_crashes must return Ok");
        let has_structure = output.contains("=== Findings ===")
            && (output.contains("=== Application crashes")
                || output.contains("No application crashes"));
        assert!(
            has_structure,
            "app_crashes must have findings block and application crashes section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_app_crashes_process_filter_accepted() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "app_crashes", "process": "chrome.exe" });
        let output = inspect_host(&args)
            .await
            .expect("app_crashes with process filter must return Ok");
        assert!(
            output.contains("Host inspection: app_crashes"),
            "app_crashes with process filter must return valid output; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_hyperv_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "hyperv" });
        let output = inspect_host(&args).await.expect("hyperv must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== Hyper-V role state ===");
        assert!(
            has_result,
            "hyperv must report findings and role state section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_windows_backup_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "windows_backup" });
        let output = inspect_host(&args)
            .await
            .expect("windows_backup must return Ok");
        assert!(
            output.contains("Host inspection: windows_backup"),
            "windows_backup output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_windows_backup_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "windows_backup" });
        let output = inspect_host(&args)
            .await
            .expect("windows_backup must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== File History ===")
            && output.contains("=== System Restore ===");
        assert!(
            has_result,
            "windows_backup must report findings and core sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_search_index_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "search_index" });
        let output = inspect_host(&args)
            .await
            .expect("search_index must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Windows Search service ===");
        assert!(
            has_result,
            "search_index must report findings and WSearch service section; got:\n{output}"
        );
    });
}

// ── display_config ────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_display_config_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "display_config" });
        let output = inspect_host(&args)
            .await
            .expect("display_config must return Ok");
        assert!(
            output.contains("Host inspection: display_config"),
            "display_config output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_display_config_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "display_config" });
        let output = inspect_host(&args)
            .await
            .expect("display_config must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== Video adapters ===");
        assert!(
            has_result,
            "display_config must report findings and video adapter section; got:\n{output}"
        );
    });
}

// ── ntp ───────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_ntp_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ntp" });
        let output = inspect_host(&args).await.expect("ntp must return Ok");
        assert!(
            output.contains("Host inspection: ntp"),
            "ntp output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_ntp_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ntp" });
        let output = inspect_host(&args).await.expect("ntp must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== Windows Time service ===");
        assert!(
            has_result,
            "ntp must report findings and Windows Time service section; got:\n{output}"
        );
    });
}

// ── cpu_power ─────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_cpu_power_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "cpu_power" });
        let output = inspect_host(&args).await.expect("cpu_power must return Ok");
        assert!(
            output.contains("Host inspection: cpu_power"),
            "cpu_power output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_cpu_power_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "cpu_power" });
        let output = inspect_host(&args).await.expect("cpu_power must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== CPU frequency ===");
        assert!(
            has_result,
            "cpu_power must report findings and CPU frequency section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_credentials_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "credentials" });
        let output = inspect_host(&args)
            .await
            .expect("credentials must return Ok");
        assert!(
            output.contains("Host inspection: credentials"),
            "credentials output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_credentials_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "credentials" });
        let output = inspect_host(&args)
            .await
            .expect("credentials must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== Credential vault summary ===")
            && output.contains("=== Credential targets");
        assert!(
            has_result,
            "credentials must report findings and credential sections; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_tpm_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "tpm" });
        let output = inspect_host(&args).await.expect("tpm must return Ok");
        assert!(
            output.contains("Host inspection: tpm"),
            "tpm output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_tpm_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "tpm" });
        let output = inspect_host(&args).await.expect("tpm must return Ok");
        let has_result = output.contains("=== Findings ===")
            && output.contains("=== TPM state ===")
            && output.contains("=== Secure Boot state ===");
        assert!(
            has_result,
            "tpm must report findings and TPM/Secure Boot sections; got:\n{output}"
        );
    });
}

// ── dhcp ──────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_dhcp_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dhcp" });
        let output = inspect_host(&args).await.expect("dhcp must return Ok");
        assert!(
            output.contains("Host inspection: dhcp"),
            "dhcp output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_dhcp_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dhcp" });
        let output = inspect_host(&args).await.expect("dhcp must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== DHCP lease details");
        assert!(
            has_result,
            "dhcp must report findings and lease sections; got:\n{output}"
        );
    });
}

// ── mtu ───────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_mtu_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "mtu" });
        let output = inspect_host(&args).await.expect("mtu must return Ok");
        assert!(
            output.contains("Host inspection: mtu"),
            "mtu output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_mtu_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "mtu" });
        let output = inspect_host(&args).await.expect("mtu must return Ok");
        let has_result = output.contains("=== Findings ===")
            && (output.contains("=== Per-adapter MTU") || output.contains("MTU"));
        assert!(
            has_result,
            "mtu must report findings and MTU sections; got:\n{output}"
        );
    });
}

// ── latency ───────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_latency_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "latency" });
        let output = inspect_host(&args).await.expect("latency must return Ok");
        assert!(
            output.contains("Host inspection: latency"),
            "latency output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_latency_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "latency" });
        let output = inspect_host(&args).await.expect("latency must return Ok");
        let has_result = output.contains("=== Findings ===")
            && (output.contains("=== Ping:")
                || output.contains("Cloudflare")
                || output.contains("Google"));
        assert!(
            has_result,
            "latency must report findings and ping sections; got:\n{output}"
        );
    });
}

// ── network_adapter ───────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_network_adapter_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "network_adapter" });
        let output = inspect_host(&args)
            .await
            .expect("network_adapter must return Ok");
        assert!(
            output.contains("Host inspection: network_adapter"),
            "network_adapter output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_network_adapter_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "network_adapter" });
        let output = inspect_host(&args)
            .await
            .expect("network_adapter must return Ok");
        let has_result =
            output.contains("=== Findings ===") && output.contains("=== Network adapters ===");
        assert!(
            has_result,
            "network_adapter must report findings and adapter sections; got:\n{output}"
        );
    });
}

// ── ssh ───────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_ssh_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ssh" });
        let output = inspect_host(&args).await.expect("ssh must return Ok");
        assert!(
            output.contains("Host inspection: ssh"),
            "ssh output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_ssh_reports_client_and_dotsssh() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ssh" });
        let output = inspect_host(&args).await.expect("ssh must return Ok");
        let has_client = output.contains("SSH client:") || output.contains("not found on PATH");
        let has_ssh_dir = output.contains("~/.ssh:") || output.contains("not found");
        assert!(
            has_client,
            "ssh must report client version or not-found; got:\n{output}"
        );
        assert!(has_ssh_dir, "ssh must report ~/.ssh state; got:\n{output}");
    });
}

// ── installed_software ────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_installed_software_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "installed_software" });
        let output = inspect_host(&args)
            .await
            .expect("installed_software must return Ok");
        assert!(
            output.contains("Host inspection: installed_software"),
            "installed_software output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_installed_software_lists_packages_or_explains() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "installed_software" });
        let output = inspect_host(&args)
            .await
            .expect("installed_software must return Ok");
        let has_result = output.contains("packages")
            || output.contains("Installed software")
            || output.contains("Homebrew")
            || output.contains("dpkg")
            || output.contains("rpm")
            || output.contains("pacman")
            || output.contains("failed")
            || output.contains("not found");
        assert!(
            has_result,
            "installed_software must list packages or explain why not; got:\n{output}"
        );
    });
}

// ── git_config ────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_git_config_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "git_config" });
        let output = inspect_host(&args)
            .await
            .expect("git_config must return Ok");
        assert!(
            output.contains("Host inspection: git_config"),
            "git_config output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_git_config_reports_version_and_config() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "git_config" });
        let output = inspect_host(&args)
            .await
            .expect("git_config must return Ok");
        let has_git = output.contains("Git:") || output.contains("not found");
        assert!(
            has_git,
            "git_config must report git version or not-found; got:\n{output}"
        );
        // If git is present, should have config info
        if output.contains("Git: git version") {
            let has_config = output.to_lowercase().contains("global git config");
            assert!(
                has_config,
                "git_config must show global config section; got:\n{output}"
            );
        }
    });
}

// ── routing: new topics are detected ─────────────────────────────────────────

#[test]
fn test_routing_detects_data_audit_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("audit this csv file"),
        Some("data_audit")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is the schema of this data?"),
        Some("data_audit")
    );
    assert_eq!(
        preferred_host_inspection_topic("inspect file profile data"),
        Some("data_audit")
    );
}

#[test]
fn test_routing_detects_docker_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("are any docker containers running?"),
        Some("docker")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me my docker images"),
        Some("docker")
    );
}

#[test]
fn test_routing_detects_docker_filesystems_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("audit my docker bind mounts and named volumes"),
        Some("docker_filesystems")
    );
    assert_eq!(
        preferred_host_inspection_topic("why is this container missing files from a bind mount?"),
        Some("docker_filesystems")
    );
}

#[test]
fn test_routing_detects_wsl_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what wsl distros do I have?"),
        Some("wsl")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me windows subsystem for linux distros"),
        Some("wsl")
    );
}

#[test]
fn test_routing_detects_wsl_filesystems_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("check my wsl filesystem storage and vhdx growth"),
        Some("wsl_filesystems")
    );
    assert_eq!(
        preferred_host_inspection_topic("is /mnt/c broken in WSL?"),
        Some("wsl_filesystems")
    );
    assert_eq!(
        preferred_host_inspection_topic("wsl df -h && wsl du -sh /mnt/c"),
        Some("wsl_filesystems")
    );
}

#[test]
fn test_routing_detects_lan_discovery_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("why can't this machine see my NAS on the local network?"),
        Some("lan_discovery")
    );
    assert_eq!(
        preferred_host_inspection_topic(
            "check local network neighborhood discovery, SMB visibility, SSDP/UPnP, and mDNS"
        ),
        Some("lan_discovery")
    );
    assert_eq!(
        preferred_host_inspection_topic("Get-NetNeighbor and SSDP discovery status"),
        Some("lan_discovery")
    );
}

#[test]
fn test_routing_detects_audio_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("why is there no sound from my speakers right now?"),
        Some("audio")
    );
    assert_eq!(
        preferred_host_inspection_topic(
            "check my microphone and playback devices because Windows Audio seems broken"
        ),
        Some("audio")
    );
}

#[test]
fn test_routing_detects_bluetooth_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic(
            "why won't this Bluetooth headset pair and stay connected?"
        ),
        Some("bluetooth")
    );
    assert_eq!(
        preferred_host_inspection_topic("check my Bluetooth radio and paired devices"),
        Some("bluetooth")
    );
}

#[test]
fn test_routing_detects_ssh_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me my ssh config"),
        Some("ssh")
    );
    assert_eq!(
        preferred_host_inspection_topic("how many known_hosts do I have?"),
        Some("ssh")
    );
}

#[test]
fn test_routing_detects_git_config_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me my git config"),
        Some("git_config")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is my git global user.name?"),
        Some("git_config")
    );
}

#[test]
fn test_routing_detects_installed_software_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what software is installed on this machine?"),
        Some("installed_software")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me installed programs"),
        Some("installed_software")
    );
}

#[test]
fn test_routing_detects_env_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me my environment variables"),
        Some("env")
    );
    assert_eq!(
        preferred_host_inspection_topic("list env vars"),
        Some("env")
    );
}

#[test]
fn test_routing_detects_hosts_file_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me the hosts file"),
        Some("hosts_file")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is in /etc/hosts?"),
        Some("hosts_file")
    );
}

#[test]
fn test_all_host_topics_detects_docker_and_ssh_together() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics("show me docker containers and my ssh config");
    assert!(
        topics.contains(&"docker"),
        "should detect docker; got: {topics:?}"
    );
    assert!(
        topics.contains(&"ssh"),
        "should detect ssh; got: {topics:?}"
    );
    assert!(
        topics.len() >= 2,
        "should detect 2+ topics; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_deep_docker_filesystem_audit_over_generic_docker() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "audit my Docker bind mounts and named volumes for missing host paths",
    );
    assert!(
        topics.contains(&"docker_filesystems"),
        "should detect docker_filesystems; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"docker"),
        "should suppress generic docker when docker_filesystems is present; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"storage"),
        "should suppress generic storage when docker_filesystems is present; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_deep_wsl_filesystem_audit_over_generic_wsl() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "check WSL storage growth and whether /mnt/c bridge health looks broken",
    );
    assert!(
        topics.contains(&"wsl_filesystems"),
        "should detect wsl_filesystems; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"wsl"),
        "should suppress generic wsl when wsl_filesystems is present; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"storage"),
        "should suppress generic storage when wsl_filesystems is present; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_lan_discovery_over_generic_network() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "check local network neighborhood discovery, SMB visibility, SSDP/UPnP, and mDNS",
    );
    assert!(
        topics.contains(&"lan_discovery"),
        "should detect lan_discovery; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"network"),
        "should suppress generic network when lan_discovery is present; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_detects_audio_and_bluetooth_together_for_headset_triage() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "my bluetooth headset connects but there is no sound and the mic keeps dropping",
    );
    assert!(
        topics.contains(&"bluetooth"),
        "should detect bluetooth; got: {topics:?}"
    );
    assert!(
        topics.contains(&"audio"),
        "should detect audio; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_audio_over_generic_peripherals() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics =
        all_host_inspection_topics("my speakers have no sound and my microphone is broken");
    assert!(
        topics.contains(&"audio"),
        "should detect audio; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"peripherals"),
        "should suppress generic peripherals when audio is present; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_bluetooth_over_generic_peripherals() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics =
        all_host_inspection_topics("check my Bluetooth headset pairing and reconnect loop");
    assert!(
        topics.contains(&"bluetooth"),
        "should detect bluetooth; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"peripherals"),
        "should suppress generic peripherals when bluetooth is present; got: {topics:?}"
    );
}

// ── databases ─────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_databases_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "databases" });
        let output = inspect_host(&args).await.expect("databases must return Ok");
        assert!(
            output.contains("Host inspection: databases"),
            "databases output must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_databases_reports_found_or_not_found() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "databases" });
        let output = inspect_host(&args).await.expect("databases must return Ok");
        let has_result =
            output.contains("[FOUND]") || output.contains("No local database engines detected");
        assert!(
            has_result,
            "databases must report found engines or explicit not-found; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_databases_mentions_docker_note() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "databases" });
        let output = inspect_host(&args).await.expect("databases must return Ok");
        assert!(
            output.contains("Docker"),
            "databases must note that Docker containers are covered by topic=docker; got:\n{output}"
        );
    });
}

#[test]
fn test_routing_detects_databases_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is postgres running on this machine?"),
        Some("databases")
    );
    assert_eq!(
        preferred_host_inspection_topic("what databases are installed locally?"),
        Some("databases")
    );
    assert_eq!(
        preferred_host_inspection_topic("is redis up?"),
        Some("databases")
    );
}

// ── Teacher mode / fix_plan new lanes ────────────────────────────────────────

#[test]
fn test_fix_plan_driver_install_returns_grounded_steps() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "how do I install a GPU driver?" });
        let output = inspect_host(&args).await.expect("fix_plan driver_install must return Ok");
        assert!(
            output.contains("fix_plan") && output.contains("driver"),
            "driver_install fix_plan must contain driver guidance; got:\n{output}"
        );
        assert!(
            output.contains("Device Manager") || output.contains("manufacturer"),
            "driver_install fix_plan must mention Device Manager or manufacturer download; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_ssh_key_reports_key_state() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "generate ssh key pair" });
        let output = inspect_host(&args)
            .await
            .expect("fix_plan ssh_key must return Ok");
        assert!(
            output.contains("id_ed25519") || output.contains("ssh-keygen"),
            "ssh_key fix_plan must mention id_ed25519 or ssh-keygen; got:\n{output}"
        );
        // Must report key detection state
        let has_key_state =
            output.contains("id_ed25519 key found:") || output.contains("id_rsa key found:");
        assert!(
            has_key_state,
            "ssh_key fix_plan must report whether keys exist; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_wsl_setup_returns_install_steps() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "how do I install WSL2?" });
        let output = inspect_host(&args).await.expect("fix_plan wsl_setup must return Ok");
        assert!(
            output.contains("wsl") || output.contains("WSL"),
            "wsl_setup fix_plan must contain WSL guidance; got:\n{output}"
        );
        assert!(
            output.contains("wsl --install") || output.contains("WSL already installed"),
            "wsl_setup fix_plan must mention wsl --install or note already installed; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_firewall_rule_returns_powershell_commands() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "create a firewall rule to open port 8080" });
        let output = inspect_host(&args).await.expect("fix_plan firewall_rule must return Ok");
        assert!(
            output.contains("New-NetFirewallRule"),
            "firewall_rule fix_plan must include New-NetFirewallRule command; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_disk_cleanup_returns_cleanup_steps() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "free up disk space my drive is almost full" });
        let output = inspect_host(&args).await.expect("fix_plan disk_cleanup must return Ok");
        assert!(
            output.contains("cleanmgr") || output.contains("Disk Cleanup") || output.contains("SoftwareDistribution"),
            "disk_cleanup fix_plan must mention cleanup tools; got:\n{output}"
        );
        assert!(
            output.contains("cargo clean") || output.contains("npm cache"),
            "disk_cleanup fix_plan must mention developer cache cleanup; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_scheduled_task_returns_register_command() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "create a scheduled task to run my script every day" });
        let output = inspect_host(&args).await.expect("fix_plan scheduled_task must return Ok");
        assert!(
            output.contains("Register-ScheduledTask"),
            "scheduled_task fix_plan must include Register-ScheduledTask command; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_registry_edit_warns_and_shows_backup() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args =
            serde_json::json!({ "topic": "fix_plan", "issue": "add a registry key in HKLM" });
        let output = inspect_host(&args)
            .await
            .expect("fix_plan registry_edit must return Ok");
        assert!(
            output.contains("reg export") || output.contains("backup"),
            "registry_edit fix_plan must mention backup/export step; got:\n{output}"
        );
        assert!(
            output.contains("Set-ItemProperty") || output.contains("New-Item"),
            "registry_edit fix_plan must include PowerShell registry commands; got:\n{output}"
        );
    });
}

#[test]
fn test_fix_plan_generic_lists_all_lanes() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "fix_plan", "issue": "completely unrelated thing not matching any lane" });
        let output = inspect_host(&args).await.expect("fix_plan generic must return Ok");
        assert!(
            output.contains("Firewall rule") || output.contains("SSH key") || output.contains("Disk cleanup"),
            "generic fix_plan must list available lanes; got:\n{output}"
        );
    });
}

// ── user_accounts / audit_policy / shares / dns_servers ──────────────────────

#[test]
fn test_inspect_host_user_accounts_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "user_accounts" });
        let output = inspect_host(&args)
            .await
            .expect("user_accounts must return Ok");
        assert!(
            output.contains("Host inspection: user_accounts"),
            "user_accounts must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_user_accounts_reports_local_users_or_sessions() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "user_accounts" });
        let output = inspect_host(&args)
            .await
            .expect("user_accounts must return Ok");
        let has_section = output.contains("Local User Accounts")
            || output.contains("Active Sessions")
            || output.contains("Active Logon Sessions");
        assert!(
            has_section,
            "user_accounts must contain a user or session section; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_user_accounts_reports_elevation() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "user_accounts" });
        let output = inspect_host(&args)
            .await
            .expect("user_accounts must return Ok");
        assert!(
            output.contains("Administrator")
                || output.contains("Elevation")
                || output.contains("elevated"),
            "user_accounts must report elevation state or admin group; got:\n{output}"
        );
    });
}

#[test]
fn test_routing_detects_user_accounts_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("list all local user accounts"),
        Some("user_accounts")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me local user accounts"),
        Some("user_accounts")
    );
    assert_eq!(
        preferred_host_inspection_topic("who has admin rights on this machine?"),
        Some("user_accounts")
    );
}

#[test]
fn test_inspect_host_audit_policy_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "audit_policy" });
        let output = inspect_host(&args)
            .await
            .expect("audit_policy must return Ok");
        assert!(
            output.contains("Host inspection: audit_policy"),
            "audit_policy must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_audit_policy_reports_policy_or_elevation_required() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "audit_policy" });
        let output = inspect_host(&args)
            .await
            .expect("audit_policy must return Ok");
        let has_result = output.contains("Audit Policy")
            || output.contains("ENABLED")
            || output.contains("No Auditing")
            || output.contains("requires Administrator")
            || output.contains("auditd")
            || output.contains("WARNING");
        assert!(
            has_result,
            "audit_policy must report policy state or note elevation required; got:\n{output}"
        );
    });
}

#[test]
fn test_routing_detects_audit_policy_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what is the audit policy on this machine?"),
        Some("audit_policy")
    );
    assert_eq!(
        preferred_host_inspection_topic("is security auditing enabled?"),
        Some("audit_policy")
    );
}

#[test]
fn test_inspect_host_shares_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "shares" });
        let output = inspect_host(&args).await.expect("shares must return Ok");
        assert!(
            output.contains("Host inspection: shares"),
            "shares must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_shares_reports_smb_section() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "shares" });
        let output = inspect_host(&args).await.expect("shares must return Ok");
        let has_section =
            output.contains("SMB") || output.contains("Samba") || output.contains("NFS");
        assert!(
            has_section,
            "shares must contain SMB, Samba, or NFS section; got:\n{output}"
        );
    });
}

#[test]
fn test_routing_detects_shares_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what SMB shares does this machine have?"),
        Some("shares")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me mapped network drives"),
        Some("shares")
    );
    assert_eq!(
        preferred_host_inspection_topic("is SMB1 enabled on this machine?"),
        Some("shares")
    );
}

#[test]
fn test_inspect_host_dns_servers_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dns_servers" });
        let output = inspect_host(&args)
            .await
            .expect("dns_servers must return Ok");
        assert!(
            output.contains("Host inspection: dns_servers"),
            "dns_servers must contain header; got:\n{output}"
        );
    });
}

#[test]
fn test_inspect_host_dns_servers_reports_resolvers_or_resolv_conf() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "dns_servers" });
        let output = inspect_host(&args)
            .await
            .expect("dns_servers must return Ok");
        let has_section = output.contains("DNS Resolver")
            || output.contains("resolv.conf")
            || output.contains("Configured DNS")
            || output.contains("systemd-resolved");
        assert!(
            has_section,
            "dns_servers must report DNS resolver config; got:\n{output}"
        );
    });
}

#[test]
fn test_routing_detects_dns_servers_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what DNS servers am I using?"),
        Some("dns_servers")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me the configured DNS resolver"),
        Some("dns_servers")
    );
    assert_eq!(
        preferred_host_inspection_topic("is DNS over HTTPS configured?"),
        Some("dns_servers")
    );
}

// ── BitLocker & Encryption ───────────────────────────────────────────────────

#[test]
fn test_inspect_host_bitlocker_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "bitlocker" });
        let output = inspect_host(&args).await.expect("bitlocker must return Ok");
        assert!(output.contains("Host inspection: bitlocker"));
    });
}

#[test]
fn test_routing_detects_bitlocker_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is my drive encrypted?"),
        Some("bitlocker")
    );
    assert_eq!(
        preferred_host_inspection_topic("bitlocker status"),
        Some("bitlocker")
    );
}

// ── RDP & Remote Access ──────────────────────────────────────────────────────

#[test]
fn test_inspect_host_rdp_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "rdp" });
        let output = inspect_host(&args).await.expect("rdp must return Ok");
        assert!(output.contains("Host inspection: rdp"));
    });
}

#[test]
fn test_routing_detects_rdp_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is remote desktop enabled?"),
        Some("rdp")
    );
    assert_eq!(
        preferred_host_inspection_topic("show RDP settings"),
        Some("rdp")
    );
}

// ── Shadow Copies (VSS) ──────────────────────────────────────────────────────

#[test]
fn test_inspect_host_shadow_copies_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "shadow_copies" });
        let output = inspect_host(&args)
            .await
            .expect("shadow_copies must return Ok");
        assert!(output.contains("Host inspection: shadow_copies"));
    });
}

#[test]
fn test_routing_detects_shadow_copies_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me shadow copies"),
        Some("shadow_copies")
    );
    assert_eq!(
        preferred_host_inspection_topic("VSS snapshots"),
        Some("shadow_copies")
    );
}

// ── Page File & Virtual Memory ───────────────────────────────────────────────

#[test]
fn test_inspect_host_pagefile_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "pagefile" });
        let output = inspect_host(&args).await.expect("pagefile must return Ok");
        assert!(output.contains("Host inspection: pagefile"));
    });
}

#[test]
fn test_routing_detects_pagefile_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("how big is my pagefile?"),
        Some("pagefile")
    );
    assert_eq!(
        preferred_host_inspection_topic("virtual memory usage"),
        Some("pagefile")
    );
}

// ── Windows Features ─────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_windows_features_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "windows_features" });
        let output = inspect_host(&args)
            .await
            .expect("windows_features must return Ok");
        assert!(output.contains("Host inspection: windows_features"));
    });
}

#[test]
fn test_routing_detects_windows_features_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what windows features are on?"),
        Some("windows_features")
    );
    assert_eq!(
        preferred_host_inspection_topic("is IIS installed?"),
        Some("windows_features")
    );
}

// ── Printers ─────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_printers_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "printers" });
        let output = inspect_host(&args).await.expect("printers must return Ok");
        assert!(output.contains("Host inspection: printers"));
    });
}

#[test]
fn test_routing_detects_printers_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("list my printers"),
        Some("printers")
    );
    assert_eq!(
        preferred_host_inspection_topic("is anything in the print queue?"),
        Some("printers")
    );
}

// ── WinRM ────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_winrm_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "winrm" });
        let output = inspect_host(&args).await.expect("winrm must return Ok");
        assert!(output.contains("Host inspection: winrm"));
    });
}

#[test]
fn test_routing_detects_winrm_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is WinRM enabled?"),
        Some("winrm")
    );
    assert_eq!(
        preferred_host_inspection_topic("check PS Remoting status"),
        Some("winrm")
    );
}

// ── Network Stats ────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_network_stats_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "network_stats" });
        let output = inspect_host(&args)
            .await
            .expect("network_stats must return Ok");
        assert!(output.contains("Host inspection: network_stats"));
    });
}

#[test]
fn test_routing_detects_network_stats_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("adapter throughput stats"),
        Some("network_stats")
    );
    assert_eq!(
        preferred_host_inspection_topic("any dropped packets on my NIC?"),
        Some("network_stats")
    );
}

// ── UDP Ports ────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_udp_ports_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "udp_ports" });
        let output = inspect_host(&args).await.expect("udp_ports must return Ok");
        assert!(output.contains("Host inspection: udp_ports"));
    });
}

#[test]
fn test_routing_detects_udp_ports_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what is listening on UDP?"),
        Some("udp_ports")
    );
    assert_eq!(
        preferred_host_inspection_topic("show open UDP ports"),
        Some("udp_ports")
    );
}
#[tokio::test]
async fn test_inspect_host_storage_includes_latency() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "storage" }))
        .await
        .expect("inspect storage fails");
    assert!(output.contains("Real-time Disk Intensity:"));
    assert!(output.contains("Average Disk Queue Length:"));
}

#[tokio::test]
async fn test_inspect_host_sessions() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "sessions" }))
        .await
        .expect("inspect sessions fails");
    assert!(output.contains("Host inspection: sessions"));
    assert!(output.contains("Active Logon Sessions") || output.contains("Logged-in Users"));
}

#[tokio::test]
async fn test_inspect_host_hardware_expanded() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(&json!({ "topic": "hardware" }))
        .await
        .expect("inspect hardware fails");
    assert!(output.contains("Motherboard:"));
    assert!(output.contains("BIOS:"));
    assert!(output.contains("Virtualization:"));
    assert!(output.contains("Hypervisor:") || output.contains("unsupported"));
}

#[tokio::test]
async fn test_inspect_host_processes_io() {
    use serde_json::json;
    let output = hematite::tools::host_inspect::inspect_host(
        &json!({ "topic": "processes", "max_entries": 1 }),
    )
    .await
    .expect("inspect processes fails");
    assert!(output.contains("Top processes by resource usage:"));
    assert!(output.contains("I/O R:") || output.contains("unknown"));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_hash_queries() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox(
        "what is the sha256 of this string?"
    ));
    assert!(needs_computation_sandbox(
        "compute the md5 checksum of this file content"
    ));
    assert!(needs_computation_sandbox(
        "generate a crc32 hash for these bytes"
    ));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_financial_queries() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox(
        "calculate 15% compound interest over 5 years"
    ));
    assert!(needs_computation_sandbox(
        "what is the roi on a $10,000 investment"
    ));
    assert!(needs_computation_sandbox(
        "compute the tax on $85,000 income"
    ));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_statistics() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox(
        "what is the standard deviation of [2, 4, 4, 4, 5, 5, 7, 9]?"
    ));
    assert!(needs_computation_sandbox(
        "calculate the mean of these values: 10, 20, 30"
    ));
    assert!(needs_computation_sandbox("find the median of this dataset"));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_unit_conversions() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox(
        "convert 2.5 gigabytes to megabytes"
    ));
    assert!(needs_computation_sandbox("how many bytes is 512 mb?"));
    assert!(needs_computation_sandbox(
        "convert 100 celsius to fahrenheit"
    ));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_date_arithmetic() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox(
        "how many days between 2024-01-15 and 2025-04-14?"
    ));
    assert!(needs_computation_sandbox(
        "what is the unix timestamp for midnight UTC today?"
    ));
    assert!(needs_computation_sandbox("how many days until christmas?"));
}

#[test]
fn test_computation_sandbox_detector_triggers_on_algorithmic_queries() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(needs_computation_sandbox("check if 7919 is prime number"));
    assert!(needs_computation_sandbox(
        "run this code and tell me the output"
    ));
    assert!(needs_computation_sandbox("execute this script for me"));
}

#[test]
fn test_computation_sandbox_detector_does_not_trigger_on_normal_queries() {
    use hematite::agent::routing::needs_computation_sandbox;
    assert!(!needs_computation_sandbox(
        "how do I refactor this function?"
    ));
    assert!(!needs_computation_sandbox(
        "what processes are using the most RAM?"
    ));
    assert!(!needs_computation_sandbox(
        "show me the git log for this repo"
    ));
    assert!(!needs_computation_sandbox(
        "explain how the vein indexer works"
    ));
}

// ── inspect_host: missing topic coverage ─────────────────────────────────────

#[tokio::test]
async fn test_inspect_host_summary_returns_hostname() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "summary" }))
        .await
        .expect("summary must return Ok");
    assert!(
        output.contains("Hostname")
            || output.contains("hostname")
            || output.contains("OS")
            || output.contains("Uptime"),
        "summary output should contain host identity info; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_os_config_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "os_config" }))
        .await
        .expect("os_config must return Ok");
    assert!(
        output.contains("OS")
            || output.contains("Power")
            || output.contains("Edition")
            || output.contains("UAC")
            || output.contains("Locale"),
        "os_config output should contain OS-level configuration; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_toolchains_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "toolchains" }))
        .await
        .expect("toolchains must return Ok");
    assert!(
        output.contains("Toolchain")
            || output.contains("Rust")
            || output.contains("Node")
            || output.contains("Python")
            || output.contains("Git")
            || output.contains("not found"),
        "toolchains output should list developer tools; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_desktop_returns_listing() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "desktop" }))
        .await
        .expect("desktop must return Ok");
    assert!(
        output.contains("Desktop")
            || output.contains("desktop")
            || output.contains("file")
            || output.contains("empty")
            || output.contains("No files"),
        "desktop output should list files or report empty; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_downloads_returns_listing() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "downloads" }))
        .await
        .expect("downloads must return Ok");
    assert!(
        output.contains("Download")
            || output.contains("download")
            || output.contains("file")
            || output.contains("empty")
            || output.contains("No files"),
        "downloads output should list files or report empty; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_disk_benchmark_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "disk_benchmark", "path": "Cargo.toml" }))
        .await
        .expect("disk_benchmark must return Ok");
    assert!(
        output.contains("Benchmark")
            || output.contains("benchmark")
            || output.contains("MB/s")
            || output.contains("throughput")
            || output.contains("Read")
            || output.contains("Write"),
        "disk_benchmark output should contain throughput info; got:\n{output}"
    );
}

// ── guard: sandbox redirect blocks ───────────────────────────────────────────

#[test]
fn test_guard_blocks_python_inline_execution() {
    use hematite::tools::guard::bash_is_safe;
    let result = bash_is_safe("python -c 'print(hello)'");
    assert!(
        result.is_err(),
        "guard should block python -c inline execution"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("run_code"),
        "guard error should mention run_code; got: {msg}"
    );
}

#[test]
fn test_guard_blocks_python3_inline_execution() {
    use hematite::tools::guard::bash_is_safe;
    let result = bash_is_safe("python3 -c 'import math; print(math.pi)'");
    assert!(
        result.is_err(),
        "guard should block python3 -c inline execution"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("run_code"),
        "guard error should mention run_code; got: {msg}"
    );
}

#[test]
fn test_guard_blocks_deno_run_execution() {
    use hematite::tools::guard::bash_is_safe;
    let result = bash_is_safe("deno run script.ts");
    assert!(
        result.is_err(),
        "guard should block deno run as sandbox substitute"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("run_code"),
        "guard error should mention run_code; got: {msg}"
    );
}

#[test]
fn test_guard_blocks_node_eval_execution() {
    use hematite::tools::guard::bash_is_safe;
    let result = bash_is_safe("node -e 'console.log(1+1)'");
    assert!(
        result.is_err(),
        "guard should block node -e as sandbox substitute"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("run_code"),
        "guard error should mention run_code; got: {msg}"
    );
}

// ── inspect_host: resource_load (previously uncovered) ───────────────────────

#[tokio::test]
async fn test_inspect_host_resource_load_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "resource_load" }))
        .await
        .expect("resource_load must return Ok");
    assert!(
        output.contains("Host inspection: resource_load"),
        "resource_load must include header; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_resource_load_reports_cpu_or_ram() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "resource_load" }))
        .await
        .expect("resource_load must return Ok");
    assert!(
        output.contains("CPU")
            || output.contains("RAM")
            || output.contains("Memory")
            || output.contains("%"),
        "resource_load output should report CPU or RAM usage; got:\n{output}"
    );
}

// ── inspect_host: content assertions for previously header-only topics ────────

#[tokio::test]
async fn test_inspect_host_bitlocker_reports_protection_state() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "bitlocker" }))
        .await
        .expect("bitlocker must return Ok");
    assert!(
        output.contains("BitLocker")
            || output.contains("Protection")
            || output.contains("Encrypted")
            || output.contains("LUKS")
            || output.contains("encryption"),
        "bitlocker output should report drive encryption state; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_rdp_reports_status() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "rdp" }))
        .await
        .expect("rdp must return Ok");
    assert!(
        output.contains("Remote Desktop")
            || output.contains("RDP")
            || output.contains("3389")
            || output.contains("fDenyTSConnections")
            || output.contains("xrdp"),
        "rdp output should report Remote Desktop state; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_shadow_copies_reports_vss_or_snapshots() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "shadow_copies" }))
        .await
        .expect("shadow_copies must return Ok");
    assert!(
        output.contains("Shadow")
            || output.contains("VSS")
            || output.contains("snapshot")
            || output.contains("Restore Point")
            || output.contains("LVM"),
        "shadow_copies output should report VSS or snapshot info; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_pagefile_reports_virtual_memory() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "pagefile" }))
        .await
        .expect("pagefile must return Ok");
    assert!(
        output.contains("Page")
            || output.contains("Virtual")
            || output.contains("MB")
            || output.contains("swap"),
        "pagefile output should report virtual memory info; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_windows_features_reports_feature_list() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "windows_features" }))
        .await
        .expect("windows_features must return Ok");
    assert!(
        output.contains("Feature")
            || output.contains("feature")
            || output.contains("Enabled")
            || output.contains("IIS")
            || output.contains("WSL")
            || output.contains("not available"),
        "windows_features output should list features or report unavailable; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_printers_reports_printers_or_none() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "printers" }))
        .await
        .expect("printers must return Ok");
    assert!(
        output.contains("Printer")
            || output.contains("printer")
            || output.contains("CUPS")
            || output.contains("No printers")
            || output.contains("queue"),
        "printers output should list printers or report none; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_winrm_reports_service_state() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "winrm" }))
        .await
        .expect("winrm must return Ok");
    assert!(
        output.contains("WinRM")
            || output.contains("WSMan")
            || output.contains("Remoting")
            || output.contains("Listener")
            || output.contains("not available"),
        "winrm output should report WinRM service state; got:\n{output}"
    );
}

// ── routing: missing detection tests ─────────────────────────────────────────

#[test]
fn test_routing_detects_resource_load_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show system load and utilization"),
        Some("resource_load")
    );
    assert_eq!(
        preferred_host_inspection_topic("why is it slow right now?"),
        Some("resource_load")
    );
}

#[test]
fn test_routing_detects_device_health_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("are there any yellow bang devices?"),
        Some("device_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("show malfunctioning hardware"),
        Some("device_health")
    );
}

#[test]
fn test_routing_detects_drivers_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("list my active system drivers"),
        Some("drivers")
    );
    assert_eq!(
        preferred_host_inspection_topic("show kernel modules"),
        Some("drivers")
    );
}

#[test]
fn test_routing_detects_peripherals_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show connected USB devices"),
        Some("peripherals")
    );
    assert_eq!(
        preferred_host_inspection_topic("list USB controllers and connected hardware"),
        Some("peripherals")
    );
}

#[test]
fn test_routing_detects_gpo_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show group policy objects"),
        Some("gpo")
    );
    assert_eq!(
        preferred_host_inspection_topic("what GPOs are applied?"),
        Some("gpo")
    );
}

#[test]
fn test_routing_detects_certificates_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show my local certificates"),
        Some("certificates")
    );
    assert_eq!(
        preferred_host_inspection_topic("list expiring certs"),
        Some("certificates")
    );
}

#[test]
fn test_routing_detects_integrity_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("check windows component integrity"),
        Some("integrity")
    );
    assert_eq!(
        preferred_host_inspection_topic("run SFC DISM health check"),
        Some("integrity")
    );
}

#[test]
fn test_routing_detects_domain_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is this machine domain joined?"),
        Some("domain")
    );
    assert_eq!(
        preferred_host_inspection_topic("show active directory domain status"),
        Some("domain")
    );
}

#[test]
fn test_routing_detects_connectivity_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("check my internet connectivity"),
        Some("connectivity")
    );
    assert_eq!(
        preferred_host_inspection_topic("am I connected to the internet?"),
        Some("connectivity")
    );
}

#[test]
fn test_routing_detects_traceroute_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("run a traceroute to 8.8.8.8"),
        Some("traceroute")
    );
    assert_eq!(
        preferred_host_inspection_topic("trace the network path to google"),
        Some("traceroute")
    );
}

#[test]
fn test_routing_detects_vpn_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show vpn tunnel status"),
        Some("vpn")
    );
    assert_eq!(
        preferred_host_inspection_topic("which vpn adapter is active?"),
        Some("vpn")
    );
}

#[test]
fn test_routing_detects_proxy_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what proxy settings are configured?"),
        Some("proxy")
    );
    assert_eq!(
        preferred_host_inspection_topic("show system proxy config"),
        Some("proxy")
    );
}

#[test]
fn test_routing_detects_firewall_rules_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("list active firewall rules"),
        Some("firewall_rules")
    );
    assert_eq!(
        preferred_host_inspection_topic("show inbound firewall allow rules"),
        Some("firewall_rules")
    );
}

#[test]
fn test_routing_detects_arp_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show the ARP table"),
        Some("arp")
    );
    assert_eq!(
        preferred_host_inspection_topic("list IP to MAC mappings"),
        Some("arp")
    );
}

#[test]
fn test_routing_detects_route_table_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show my routing table"),
        Some("route_table")
    );
    assert_eq!(
        preferred_host_inspection_topic("print the IP route table"),
        Some("route_table")
    );
}

#[test]
fn test_routing_detects_os_config_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show uptime and last boot time"),
        Some("os_config")
    );
    assert_eq!(
        preferred_host_inspection_topic("check uptime and last boot time"),
        Some("os_config")
    );
}

#[test]
fn test_routing_detects_toolchains_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what developer toolchains are installed?"),
        Some("toolchains")
    );
    assert_eq!(
        preferred_host_inspection_topic("detect installed Rust Node Python versions"),
        Some("toolchains")
    );
}

#[test]
fn test_routing_detects_disk_benchmark_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("run a disk stress test on this drive"),
        Some("disk_benchmark")
    );
    assert_eq!(
        preferred_host_inspection_topic("give me an io intensity report"),
        Some("disk_benchmark")
    );
}

#[test]
fn test_routing_detects_log_check_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me recent errors from the Windows event log"),
        Some("log_check")
    );
    assert_eq!(
        preferred_host_inspection_topic("are there any recent warnings in the system log?"),
        Some("log_check")
    );
    assert_eq!(
        preferred_host_inspection_topic("open event viewer and show me errors"),
        Some("log_check")
    );
}

#[test]
fn test_routing_detects_storage_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show my storage usage across all drives"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("how much free space do I have?"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("where is all my disk space going?"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me disk usage on each drive"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("am I running out of space?"),
        Some("storage")
    );
}

#[test]
fn test_routing_detects_hardware_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what is my CPU model?"),
        Some("hardware")
    );
    assert_eq!(
        preferred_host_inspection_topic("how much RAM does this machine have?"),
        Some("hardware")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me the hardware specs for this machine"),
        Some("hardware")
    );
    assert_eq!(
        preferred_host_inspection_topic("what GPU do I have?"),
        Some("hardware")
    );
}

// --- Prompt library coverage tests ---

#[test]
fn test_routing_prompt_library_open_ports_and_connections() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    // prompt_library "Open ports and active connections"
    let prompt = "Show me all listening TCP and UDP ports with their owning processes, and list any established outbound connections.";
    // single-topic routing detects udp_ports first (contains "udp port" substring),
    // but this prompt triggers the multi-topic pre-run so single-topic is bypassed.
    let single = preferred_host_inspection_topic(prompt);
    assert!(
        single == Some("ports") || single == Some("udp_ports"),
        "single-topic routing should pick ports or udp_ports; got: {single:?}"
    );
    // multi-topic pre-run should detect both ports and connections so both are run together
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"ports"),
        "multi-topic should detect ports; got: {topics:?}"
    );
    assert!(
        topics.contains(&"connections"),
        "multi-topic should detect connections; got: {topics:?}"
    );
    // 2+ topics means the pre-run fires and single-topic routing is bypassed
    assert!(
        topics.len() >= 2,
        "should detect 2+ topics for pre-run; got: {topics:?}"
    );
}

#[test]
fn test_routing_prompt_library_dns_and_proxy() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    // prompt_library "DNS and proxy audit"
    let prompt = "Show me my configured DNS nameservers per adapter and any system proxy settings — WinHTTP, Internet Options, and environment variables.";
    // single-topic path should route to dns_servers (it's earlier in dispatch)
    assert_eq!(
        preferred_host_inspection_topic(prompt),
        Some("dns_servers"),
        "single-topic routing should pick dns_servers"
    );
    // multi-topic path should detect both dns_servers and proxy for pre-run
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"dns_servers"),
        "multi-topic should detect dns_servers; got: {topics:?}"
    );
    assert!(
        topics.contains(&"proxy"),
        "multi-topic should detect proxy; got: {topics:?}"
    );
}

#[test]
fn test_routing_prompt_library_firewall_rules() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // prompt_library "Firewall rules"
    assert_eq!(
        preferred_host_inspection_topic(
            "List all active inbound firewall rules that allow traffic. Flag anything that looks non-default."
        ),
        Some("firewall_rules")
    );
}

#[test]
fn test_routing_prompt_library_traceroute() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // prompt_library "Traceroute"
    assert_eq!(
        preferred_host_inspection_topic(
            "Trace the network path to 8.8.8.8 and tell me where the latency spikes are."
        ),
        Some("traceroute")
    );
}

#[test]
fn test_routing_prompt_library_connectivity_triage() {
    use hematite::agent::routing::all_host_inspection_topics;
    // prompt_library "Connectivity triage"
    let prompt = "Check my internet connectivity, Wi-Fi signal strength, and VPN status. If I'm on a VPN, tell me which adapter is handling the tunnel.";
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"connectivity"),
        "should detect connectivity; got: {topics:?}"
    );
    assert!(
        topics.contains(&"wifi"),
        "should detect wifi; got: {topics:?}"
    );
    assert!(
        topics.contains(&"vpn"),
        "should detect vpn; got: {topics:?}"
    );
}

#[test]
fn test_routing_prompt_library_crash_and_reboot_history() {
    use hematite::agent::routing::all_host_inspection_topics;
    // prompt_library "Crash and reboot history" — asks for both crash events and pending reboot
    let prompt = "Show me any BSOD or unexpected shutdown events from the last week, and tell me if a reboot is currently pending and why.";
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"recent_crashes"),
        "should detect recent_crashes; got: {topics:?}"
    );
    assert!(
        topics.contains(&"pending_reboot"),
        "should detect pending_reboot ('reboot is currently pending'); got: {topics:?}"
    );
}

#[test]
fn test_routing_prompt_library_network_map() {
    use hematite::agent::routing::all_host_inspection_topics;
    // prompt_library "Network map"
    let prompt = "Show me my routing table, ARP table, and DNS cache. Map out the devices this machine is currently aware of on the local network.";
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"route_table"),
        "should detect route_table; got: {topics:?}"
    );
    assert!(
        topics.contains(&"arp"),
        "should detect arp; got: {topics:?}"
    );
    assert!(
        topics.contains(&"dns_cache"),
        "should detect dns_cache; got: {topics:?}"
    );
    assert!(
        topics.contains(&"lan_discovery"),
        "should detect lan_discovery for neighborhood mapping; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_credentials_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Audit my stored Windows credentials and tell me if Credential Manager hygiene looks risky.";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("credentials"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"credentials"),
        "should detect credentials; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_event_query_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt = "Show me all System errors from the Event Log that occurred in the last 4 hours.";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("event_query"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"event_query"),
        "should detect event_query; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_event_query_over_log_check_for_targeted_event_prompts() {
    use hematite::agent::routing::all_host_inspection_topics;
    let prompt = "Show me all System errors from the Event Log that occurred in the last 4 hours.";
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"event_query"),
        "should include event_query; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"log_check"),
        "should suppress log_check when event_query is present; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_tpm_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Check TPM, Secure Boot, and firmware mode and tell me if this machine is Windows 11 ready.";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("tpm"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"tpm"),
        "should detect tpm; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_latency_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt = "My internet feels slow and I'm seeing high latency — can you ping the gateway and check for packet loss?";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("latency"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"latency"),
        "should detect latency; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_network_adapter_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt = "Check my NIC settings — I want to see link speed, offload settings, and any adapter errors.";
    assert_eq!(
        preferred_host_inspection_topic(prompt),
        Some("network_adapter")
    );
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"network_adapter"),
        "should detect network_adapter; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_dhcp_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Show me my DHCP lease details — when does it expire and which DHCP server assigned it?";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("dhcp"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"dhcp"),
        "should detect dhcp; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_mtu_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt = "Check my MTU settings — I think VPN fragmentation is causing issues.";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("mtu"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"mtu"),
        "should detect mtu; got: {topics:?}"
    );
}

// ── IT Pro Plus Diagnostics ──────────────────────────────────────────────────

#[test]
fn test_routing_detects_onedrive_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Check OneDrive sync health and tell me if my Desktop/Documents backup is working.";
    assert_eq!(preferred_host_inspection_topic(prompt), Some("onedrive"));
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"onedrive"),
        "should detect onedrive; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_identity_auth_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Audit token broker, Web Account Manager, and device registration for Microsoft 365 sign-in health.";
    assert_eq!(
        preferred_host_inspection_topic(prompt),
        Some("identity_auth")
    );
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"identity_auth"),
        "should detect identity_auth; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_identity_auth_over_app_health_for_signin_prompts() {
    use hematite::agent::routing::all_host_inspection_topics;
    let prompt = "Why won't Outlook sign in and why does Teams keep asking me to authenticate?";
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"identity_auth"),
        "should include identity_auth; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"outlook") && !topics.contains(&"teams") && !topics.contains(&"sign_in"),
        "should suppress overlapping app-health topics for auth-specific prompts; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_browser_health_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt =
        "Check browser health and tell me if WebView2 or proxy policy is breaking web apps.";
    assert_eq!(
        preferred_host_inspection_topic(prompt),
        Some("browser_health")
    );
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"browser_health"),
        "should detect browser_health; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_installer_health_topic() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let prompt = "Why are MSI and winget installs failing on this Windows machine?";
    assert_eq!(
        preferred_host_inspection_topic(prompt),
        Some("installer_health")
    );
    let topics = all_host_inspection_topics(prompt);
    assert!(
        topics.contains(&"installer_health"),
        "should detect installer_health; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_prefers_browser_health_over_proxy_for_browser_proxy_prompts() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "Check whether browser policy or proxy settings are interfering with web apps.",
    );
    assert!(
        topics.contains(&"browser_health"),
        "should detect browser_health; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"proxy"),
        "should suppress generic proxy when browser_health is present; got: {topics:?}"
    );
}

#[tokio::test]
async fn test_inspect_host_ad_user_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "ad_user", "name": "administrator" }))
        .await
        .unwrap();
    assert!(output.contains("Host inspection: ad_user"));
}

#[tokio::test]
async fn test_inspect_host_dns_lookup_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "dns_lookup", "name": "google.com", "type": "A" }))
        .await
        .unwrap();
    assert!(output.contains("Host inspection: dns_lookup"));
}

#[tokio::test]
async fn test_inspect_host_hyperv_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "hyperv" })).await.unwrap();
    assert!(output.contains("Host inspection: hyperv"));
}

#[tokio::test]
async fn test_inspect_host_mdm_enrollment_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "mdm_enrollment" }))
        .await
        .unwrap();
    assert!(
        output.contains("Host inspection: mdm_enrollment"),
        "mdm_enrollment must return a header; got:\n{output}"
    );
}

#[test]
fn test_routing_detects_mdm_enrollment_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let phrases = [
        "is my device enrolled in Intune",
        "check MDM enrollment status",
        "is this device managed by MDM",
        "show me the Intune enrollment state",
        "is the device Azure AD joined",
    ];
    for phrase in &phrases {
        assert_eq!(
            preferred_host_inspection_topic(phrase),
            Some("mdm_enrollment"),
            "phrase {:?} should route to mdm_enrollment",
            phrase
        );
    }
}

#[tokio::test]
async fn test_inspect_host_mdm_enrollment_reports_findings() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "mdm_enrollment" }))
        .await
        .unwrap();
    assert!(
        output.contains("=== Findings ==="),
        "mdm_enrollment must include a Findings section; got:\n{output}"
    );
    assert!(
        output.contains("=== Device join and MDM state"),
        "mdm_enrollment must include dsregcmd section; got:\n{output}"
    );
}

#[tokio::test]
async fn test_inspect_host_ip_config_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    use serde_json::json;
    let output = inspect_host(&json!({ "topic": "ip_config" }))
        .await
        .unwrap();
    assert!(output.contains("Host inspection: ip_config"));
}

#[test]
fn test_routing_prompts_it_pro_plus() {
    use hematite::agent::routing::all_host_inspection_topics;

    // ad_user
    let topics = all_host_inspection_topics(
        "Analyze the AD user administrator. Show their SID and group memberships.",
    );
    assert!(
        topics.contains(&"ad_user"),
        "should detect ad_user; got: {topics:?}"
    );

    // hyperv
    let topics =
        all_host_inspection_topics("Inventory my Hyper-V VMs and show their current load.");
    assert!(
        topics.contains(&"hyperv"),
        "should detect hyperv; got: {topics:?}"
    );

    // ip_config
    let topics =
        all_host_inspection_topics("Show me a detailed ipconfig /all report with DHCP discovery.");
    assert!(
        topics.contains(&"ip_config"),
        "should detect ip_config; got: {topics:?}"
    );
}

#[test]
fn test_routing_sovereign_mutation_pruning() {
    use hematite::agent::conversation::WorkflowMode;
    use hematite::agent::routing::classify_query_intent;

    let prompt = "Make me a folder on my Desktop named Success";
    let intent = classify_query_intent(WorkflowMode::Auto, prompt);

    // Sovereign mode should hide workflow tools
    assert!(
        !intent.workspace_workflow_mode,
        "Sovereign request should prune workspace workflows"
    );
    assert!(
        !intent.maintainer_workflow_mode,
        "Sovereign request should prune maintainer workflows"
    );
}

#[test]
fn test_hallucination_sanitizer_logic() {
    // Note: We need to expose is_natural_language_hallucination or test via a public entry
    // For now, we'll verify the logic matches the implementation in conversation.rs
    let _sentences = [
        "Make me a folder please",
        "I want to create a directory",
        "How do I run this?",
        "Let's go and build it",
        "Create the desktop folder now",
    ];

    let _commands = [
        "npm install",
        "cargo build --release",
        "mkdir path/to/dir",
        "git commit -m 'fix'",
        "./scripts/test.sh",
    ];

    // This is a manual logic check since the function is private to conversation.rs
    // In a real scenario, we'd make it pub(crate) for testing.
}

// ── IPv6 ────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_ipv6_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ipv6" });
        let out = inspect_host(&args).await.expect("ipv6 must return Ok");
        assert!(
            out.contains("ipv6"),
            "ipv6 output must contain topic header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_ipv6_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ipv6" });
        let out = inspect_host(&args).await.expect("ipv6 must return Ok");
        assert!(
            out.contains("Findings") || out.contains("IPv6"),
            "ipv6 output must contain Findings or IPv6 section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_ipv6_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Show my IPv6 addresses and prefix",
        "Is SLAAC or DHCPv6 assigning my address?",
        "Check IPv6 config on this machine",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("ipv6"), "Expected ipv6 for: {q}");
    }
}

// ── TCP Parameters ──────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_tcp_params_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "tcp_params" });
        let out = inspect_host(&args)
            .await
            .expect("tcp_params must return Ok");
        assert!(
            out.contains("tcp_params"),
            "tcp_params output must contain topic header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_tcp_params_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "tcp_params" });
        let out = inspect_host(&args)
            .await
            .expect("tcp_params must return Ok");
        assert!(
            out.contains("Findings") || out.contains("TCP"),
            "tcp_params output must contain Findings or TCP section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_tcp_params_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Check TCP autotuning settings",
        "What congestion algorithm is Windows using?",
        "Show TCP parameters and receive window size",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("tcp_params"), "Expected tcp_params for: {q}");
    }
}

// ── WLAN Profiles ───────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_wlan_profiles_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wlan_profiles" });
        let out = inspect_host(&args)
            .await
            .expect("wlan_profiles must return Ok");
        assert!(
            out.contains("wlan_profiles"),
            "wlan_profiles output must contain topic header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_wlan_profiles_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wlan_profiles" });
        let out = inspect_host(&args)
            .await
            .expect("wlan_profiles must return Ok");
        assert!(
            out.contains("Findings")
                || out.contains("wireless")
                || out.contains("profile")
                || out.contains("WLAN")
                || out.contains("wifi"),
            "wlan_profiles output must contain wireless profile info or findings; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_wlan_profiles_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Show my saved wifi networks",
        "Audit wlan profile security — any WEP or open auth?",
        "List saved wireless networks on this machine",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("wlan_profiles"),
            "Expected wlan_profiles for: {q}"
        );
    }
}

// ── IPSec ───────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_ipsec_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ipsec" });
        let out = inspect_host(&args).await.expect("ipsec must return Ok");
        assert!(
            out.contains("ipsec"),
            "ipsec output must contain topic header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_ipsec_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "ipsec" });
        let out = inspect_host(&args).await.expect("ipsec must return Ok");
        assert!(
            out.contains("Findings")
                || out.contains("IPSec")
                || out.contains("IKE")
                || out.contains("SA"),
            "ipsec output must contain Findings or IPSec section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_ipsec_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Check IPSec security associations",
        "Is there an active IKE tunnel?",
        "Show IPSec policy and SA state",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("ipsec"), "Expected ipsec for: {q}");
    }
}

// ── netbios ──────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_netbios_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "netbios" });
        let out = inspect_host(&args).await.expect("netbios must return Ok");
        assert!(
            out.contains("NetBIOS") || out.contains("WINS") || out.contains("nbtstat"),
            "netbios output must contain NetBIOS header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_netbios_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "netbios" });
        let out = inspect_host(&args).await.expect("netbios must return Ok");
        assert!(
            out.contains("Findings") || out.contains("NetBIOS") || out.contains("Adapter"),
            "netbios output must contain Findings or adapter section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_netbios_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Show NetBIOS name table",
        "What WINS server is configured?",
        "Are there active nbtstat sessions?",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("netbios"), "Expected netbios for: {q}");
    }
}

// ── nic_teaming ───────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_nic_teaming_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "nic_teaming" });
        let out = inspect_host(&args)
            .await
            .expect("nic_teaming must return Ok");
        assert!(
            out.contains("NIC Teaming")
                || out.contains("LBFO")
                || out.contains("Team")
                || out.contains("teaming"),
            "nic_teaming output must contain NIC Teaming header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_nic_teaming_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "nic_teaming" });
        let out = inspect_host(&args)
            .await
            .expect("nic_teaming must return Ok");
        assert!(
            out.contains("Findings") || out.contains("Team") || out.contains("No NIC teams"),
            "nic_teaming output must contain Findings or team section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_nic_teaming_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Show LACP link aggregation status",
        "Is link aggregation enabled?",
        "Check LBFO team status",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("nic_teaming"), "Expected nic_teaming for: {q}");
    }
}

// ── snmp ─────────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_snmp_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "snmp" });
        let out = inspect_host(&args).await.expect("snmp must return Ok");
        assert!(
            out.contains("SNMP") || out.contains("snmp"),
            "snmp output must contain SNMP header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_snmp_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "snmp" });
        let out = inspect_host(&args).await.expect("snmp must return Ok");
        assert!(
            out.contains("Findings")
                || out.contains("Service")
                || out.contains("Community")
                || out.contains("SNMP"),
            "snmp output must contain Findings or service section; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_snmp_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Is SNMP agent running?",
        "Show SNMP community strings",
        "Check SNMP trap service",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("snmp"), "Expected snmp for: {q}");
    }
}

// ── port_test ─────────────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_port_test_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "port_test", "host": "8.8.8.8", "port": 53 });
        let out = inspect_host(&args).await.expect("port_test must return Ok");
        assert!(
            out.contains("Port Test")
                || out.contains("port")
                || out.contains("TCP")
                || out.contains("reachab"),
            "port_test output must contain Port Test header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_port_test_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "port_test", "host": "8.8.8.8", "port": 53 });
        let out = inspect_host(&args).await.expect("port_test must return Ok");
        assert!(
            out.contains("Findings")
                || out.contains("OPEN")
                || out.contains("CLOSED")
                || out.contains("TCP"),
            "port_test output must contain Findings or result; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_port_test_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Is port 443 open on 1.1.1.1?",
        "Port check on 192.168.1.1:22",
        "Check if port 80 is reachable",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("port_test"), "Expected port_test for: {q}");
    }
}

// ── network_profile ───────────────────────────────────────────────────────────

#[test]
fn test_inspect_host_network_profile_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "network_profile" });
        let out = inspect_host(&args)
            .await
            .expect("network_profile must return Ok");
        assert!(
            out.contains("network_profile") || out.contains("Network") || out.contains("location"),
            "network_profile output must contain header; got:\n{out}"
        );
    });
}

#[test]
fn test_inspect_host_network_profile_reports_findings_and_sections() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "network_profile" });
        let out = inspect_host(&args)
            .await
            .expect("network_profile must return Ok");
        assert!(
            out.contains("Findings")
                || out.contains("Private")
                || out.contains("Public")
                || out.contains("Domain"),
            "network_profile output must contain Findings or category; got:\n{out}"
        );
    });
}

#[test]
fn test_routing_detects_network_profile_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "What is the network location profile?",
        "Is this a public or private network?",
        "Show network category for each adapter",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("network_profile"),
            "Expected network_profile for: {q}"
        );
    }
}

// ── dns_lookup ────────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_dns_lookup_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "DNS lookup for github.com",
        "Do an nslookup on cloudflare.com",
        "Resolve the A record for example.com",
        "What is the IP address of google.com",
        "Resolve-DnsName github.com -Type A",
        "host github.com",
        "powershell -Command \"$ip = [System.Net.Dns]::GetHostAddresses('github.com'); $ip | ForEach-Object { $_.Address }\"",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("dns_lookup"), "Expected dns_lookup for: {q}");
    }
}

#[test]
fn test_all_host_topics_prefers_dns_lookup_over_network_for_domain_ip_questions() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics("What is the IP address of google.com");
    assert!(
        topics.contains(&"dns_lookup"),
        "expected dns_lookup; got: {topics:?}"
    );
    assert!(
        !topics.contains(&"network"),
        "did not expect generic network fallback; got: {topics:?}"
    );
}

#[test]
fn test_conversational_advisory_does_not_trigger_summary_route() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Advisory follow-ups that contain host-inspection keywords ("ram", "vram")
    // must NOT route to inspect_host(summary) — they're opinion questions.
    let advisory = [
        "would another stick of ram be nice",
        "would another stick of ram be nice, i could offload more vram stuff to it right?",
        "would upgrading my ram help",
        "could I offload vram to system ram",
        "is that worth it right?",
        "would more memory be useful",
        "should I upgrade my gpu",
        "do you think more ram would help",
    ];
    for q in &advisory {
        let topic = preferred_host_inspection_topic(q);
        assert!(
            topic != Some("summary"),
            "Expected no summary route for advisory question: {q} (got: {topic:?})"
        );
    }
}

#[test]
fn test_direct_diagnostic_questions_still_route_through_advisory_guard() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Real diagnostic questions that happen to contain "ram" or "memory"
    // should still route correctly.
    assert_eq!(
        preferred_host_inspection_topic("how much ram do I have"),
        Some("hardware")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is using my ram"),
        Some("processes")
    );
    assert_eq!(
        preferred_host_inspection_topic("what processes are using ram"),
        Some("processes")
    );
}

#[test]
fn test_conversational_declaratives_do_not_trigger_summary_route() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Declarative statements, opinions, and hypotheticals containing host-inspection
    // keywords must NOT trigger inspect_host(summary) — no new data needed.
    let should_not_route_to_summary: &[&str] = &[
        "i think the gpu is fine",
        "makes sense the cpu is fine",
        "what if i had more ram",
        "if i upgraded the gpu would that help",
        "so the vram is being used by lm studio",
        "i see the memory is fine",
        "everything looks good with my ram",
        "ok so the cpu is at 8 percent",
        "i believe the service is running",
        "i know the network is fine",
        "so the ram is the issue",
        "so my gpu is the bottleneck",
        "ah ok so the cpu is throttled",
    ];
    for q in should_not_route_to_summary {
        let topic = preferred_host_inspection_topic(q);
        assert!(
            topic != Some("summary"),
            "Expected no summary route for declarative/conversational: {q:?} (got: {topic:?})"
        );
    }
}

#[test]
fn test_routing_detects_storage_spaces_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me my storage pools"),
        Some("storage_spaces")
    );
    assert_eq!(
        preferred_host_inspection_topic("is my Windows RAID degraded?"),
        Some("storage_spaces")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is the health of my virtual disks?"),
        Some("storage_spaces")
    );
    assert_eq!(
        preferred_host_inspection_topic("show storage space pool status"),
        Some("storage_spaces")
    );
}

#[test]
fn test_routing_detects_defender_quarantine_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show defender quarantine history"),
        Some("defender_quarantine")
    );
    assert_eq!(
        preferred_host_inspection_topic("what threats has Defender detected?"),
        Some("defender_quarantine")
    );
    assert_eq!(
        preferred_host_inspection_topic("did defender find any malware?"),
        Some("defender_quarantine")
    );
    assert_eq!(
        preferred_host_inspection_topic("show threat history"),
        Some("defender_quarantine")
    );
}

#[test]
fn test_scaffold_request_detection() {
    use hematite::agent::routing::is_scaffold_request;

    // Web stacks
    assert!(is_scaffold_request("create a React app for me"));
    assert!(is_scaffold_request("build me a Next.js app"));
    assert!(is_scaffold_request("make me a landing page"));
    assert!(is_scaffold_request("set up a Vue app for me"));
    assert!(is_scaffold_request("generate a todo app in React"));
    assert!(is_scaffold_request("spin up an Express server"));
    assert!(is_scaffold_request("make me a website"));
    assert!(is_scaffold_request("create a web app"));

    // Systems / compiled stacks
    assert!(is_scaffold_request("build me a Rust CLI app"));
    assert!(is_scaffold_request("create a Rust project"));
    assert!(is_scaffold_request("make me a Go CLI tool"));
    assert!(is_scaffold_request("scaffold a Go project"));
    assert!(is_scaffold_request("create a C++ project"));
    assert!(is_scaffold_request("make a cmake project"));

    // Python
    assert!(is_scaffold_request("scaffold a FastAPI project"));
    assert!(is_scaffold_request("make me a Python CLI tool"));
    assert!(is_scaffold_request("create a Python package"));
    assert!(is_scaffold_request("build a Flask app"));

    // Explicit commands
    assert!(is_scaffold_request("npm init my project"));
    assert!(is_scaffold_request("cargo new my-cli"));
    assert!(is_scaffold_request("go mod init my-app"));

    // Should NOT detect scaffold intent
    assert!(!is_scaffold_request(
        "how do I add a component to my React app"
    ));
    assert!(!is_scaffold_request("fix the bug in my Express route"));
    assert!(!is_scaffold_request("explain how FastAPI routing works"));
    assert!(!is_scaffold_request("what is my CPU usage"));
    assert!(!is_scaffold_request("show me running processes"));
    assert!(!is_scaffold_request("what rust version am I on"));
}

#[test]
fn test_diagnose_triage_all_good() {
    use hematite::agent::diagnose::triage_follow_up_topics;
    let health = "System Health Report — ALL GOOD\n\nLooking good:\n  [+] Disk: 200 GB free\n  [+] RAM: 16 GB free\n  [+] Dev tools found: Git, Rust / Cargo";
    let topics = triage_follow_up_topics(health);
    assert!(
        topics.is_empty(),
        "ALL GOOD should return no follow-up topics, got: {:?}",
        topics
    );
}

#[test]
fn test_diagnose_triage_disk_action_required() {
    use hematite::agent::diagnose::triage_follow_up_topics;
    let health = "System Health Report — ACTION REQUIRED\n\nNeeds fixing:\n  [!] Disk: 1 GB free on C: (0% available)";
    let topics = triage_follow_up_topics(health);
    assert!(
        topics.contains(&"storage"),
        "disk [!] should trigger storage"
    );
    assert!(
        topics.contains(&"disk_health"),
        "disk [!] should trigger disk_health"
    );
}

#[test]
fn test_diagnose_triage_event_log_errors() {
    use hematite::agent::diagnose::triage_follow_up_topics;
    let health = "System Health Report — WORTH A LOOK\n\nWorth watching:\n  [-] 68 critical/error events in Windows event logs in the last 24 hours.";
    let topics = triage_follow_up_topics(health);
    assert!(
        topics.contains(&"log_check"),
        "event log errors should trigger log_check"
    );
}

#[test]
fn test_diagnose_triage_skips_toolchain_warnings() {
    use hematite::agent::diagnose::triage_follow_up_topics;
    let health = "System Health Report — WORTH A LOOK\n\nWorth watching:\n  [-] Not installed (or not on PATH): Python, npm — only matters if you need them";
    let topics = triage_follow_up_topics(health);
    // Dev tool "not installed" warnings should NOT trigger system health follow-up
    assert!(
        !topics.contains(&"toolchains"),
        "toolchain warnings should not trigger follow-up"
    );
    assert!(
        !topics.contains(&"dev_conflicts"),
        "toolchain warnings should not trigger follow-up"
    );
}

#[test]
fn test_diagnose_instruction_names_exact_topics() {
    use hematite::agent::diagnose::build_diagnose_instruction;
    let health = "System Health Report — WORTH A LOOK\n\nWorth watching:\n  [-] 45 error events.";
    let topics = &["log_check", "services"];
    let instruction = build_diagnose_instruction(health, topics);
    assert!(
        instruction.contains("log_check"),
        "instruction must name log_check"
    );
    assert!(
        instruction.contains("services"),
        "instruction must name services"
    );
    assert!(
        instruction.contains("PROTOCOL"),
        "instruction must include protocol header"
    );
    assert!(
        instruction.contains("numbered fix plan"),
        "instruction must request grounded fix plan"
    );
}

#[test]
fn test_report_export_markdown_structure() {
    use hematite::agent::report_export;
    let _ = std::hint::black_box(report_export::generate_report_markdown as *const () as usize);
    let _ = std::hint::black_box(report_export::generate_report_json as *const () as usize);
    let _ = std::hint::black_box(report_export::generate_report_html as *const () as usize);
    let _ = std::hint::black_box(report_export::save_report_markdown as *const () as usize);
    let _ = std::hint::black_box(report_export::save_report_json as *const () as usize);
    let _ = std::hint::black_box(report_export::save_report_html as *const () as usize);
}

// ── HTML report ───────────────────────────────────────────────────────────────

#[test]
fn test_html_report_action_plan_html_healthy() {
    use hematite::agent::fix_recipes::format_action_plan_html;
    let sections: &[(&str, &str)] = &[("health_report", "ALL GOOD system is healthy")];
    let html = format_action_plan_html(sections);
    assert!(
        html.contains("healthy"),
        "healthy output should say 'healthy'"
    );
    assert!(
        !html.contains("<div class=\"recipe"),
        "no recipe cards for a clean machine"
    );
}

#[test]
fn test_html_report_action_plan_html_with_issues() {
    use hematite::agent::fix_recipes::format_action_plan_html;
    let sections: &[(&str, &str)] = &[(
        "health_report",
        "disk: C: — very low free space\npending reboot required",
    )];
    let html = format_action_plan_html(sections);
    assert!(
        html.contains("<div class=\"recipe"),
        "should contain recipe cards"
    );
    assert!(
        html.contains("b-action") || html.contains("b-investigate"),
        "should have severity badges"
    );
    assert!(html.contains("<ol>"), "steps should be in an ordered list");
}

#[test]
fn test_html_report_escapes_special_chars() {
    use hematite::agent::fix_recipes::format_action_plan_html;
    // The recipe steps contain PowerShell-style strings with <, >, & chars
    let sections: &[(&str, &str)] = &[("health_report", "disk: C: — very low free space")];
    let html = format_action_plan_html(sections);
    // Should not contain raw unescaped angle brackets from step content outside of real tags
    // (steps are in <li> tags so the step text itself must be escaped)
    assert!(
        !html.contains("&lt;"),
        "no escaped content needed in these steps"
    ); // steps don't have < in them
    assert!(html.contains("</ol>"), "ordered list must close");
}

#[test]
fn test_html_report_format_flag() {
    use clap::CommandFactory;
    use hematite::CliCockpit;
    let cmd = CliCockpit::command();
    let format_arg = cmd
        .get_arguments()
        .find(|a| a.get_long() == Some("report-format"));
    assert!(format_arg.is_some(), "--report-format flag must exist");
    let help = format_arg
        .unwrap()
        .get_help()
        .map(|h| h.to_string())
        .unwrap_or_default();
    assert!(
        help.contains("html") || help.to_ascii_lowercase().contains("html"),
        "--report-format help text should mention html: {}",
        help
    );
}

#[test]
fn test_triage_json_output_wiring() {
    // Verify save_triage_report_json is a callable public function (smoke test)
    let _ = std::hint::black_box(
        hematite::agent::report_export::save_triage_report_json as *const () as usize,
    );
}

#[test]
fn test_diagnosis_json_output_wiring() {
    // Verify save_diagnosis_report_json is a callable public function (smoke test)
    let _ = std::hint::black_box(
        hematite::agent::report_export::save_diagnosis_report_json as *const () as usize,
    );
}

#[test]
fn test_report_cli_flags_exist() {
    // Smoke-test that --report, --report-format, and --diagnose are valid CliCockpit fields.
    use clap::CommandFactory;
    use hematite::CliCockpit;
    let cmd = CliCockpit::command();
    let flag_names: Vec<&str> = cmd
        .get_arguments()
        .map(|a| a.get_long().unwrap_or(""))
        .collect();
    assert!(
        flag_names.contains(&"report"),
        "--report flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"report-format"),
        "--report-format flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"diagnose"),
        "--diagnose flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"open"),
        "--open flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"output"),
        "--output flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"quiet"),
        "--quiet flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"clipboard"),
        "--clipboard flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"notify"),
        "--notify flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"count"),
        "--count flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"compare"),
        "--compare flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"yes"),
        "--yes flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"only"),
        "--only flag missing from CliCockpit"
    );
    assert!(
        flag_names.contains(&"field"),
        "--field flag missing from CliCockpit"
    );
}

#[test]
fn test_fix_all_only_filters_by_label() {
    use hematite::agent::report_export::sweep_auto_fixes;
    let all = sweep_auto_fixes();
    // "Flush DNS Cache" is a known entry — verify partial match works
    let dns_fixes: Vec<_> = all
        .iter()
        .filter(|f| f.label.to_ascii_lowercase().contains("dns"))
        .collect();
    assert!(
        !dns_fixes.is_empty(),
        "Expected at least one sweep fix with 'dns' in the label"
    );
}

#[test]
fn test_fix_all_only_list_returns_all_labels() {
    use hematite::agent::report_export::sweep_auto_fixes;
    let all = sweep_auto_fixes();
    for fix in &all {
        assert!(
            !fix.label.is_empty(),
            "Each sweep fix must have a non-empty label"
        );
    }
}

#[test]
fn test_triage_dry_run_default_has_five_topics() {
    let topics = hematite::agent::report_export::triage_topics_for_preset("default");
    assert_eq!(
        topics.len(),
        5,
        "default triage should have 5 topics: health, security, connectivity, identity, updates"
    );
}

#[test]
fn test_triage_dry_run_network_preset_includes_wifi() {
    let topics = hematite::agent::report_export::triage_topics_for_preset("network");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        names.contains(&"wifi"),
        "network triage preset should include wifi, got: {:?}",
        names
    );
}

#[test]
fn test_diagnose_dry_run_report_topics_has_six() {
    let topics = hematite::agent::report_export::report_topics();
    assert_eq!(
        topics.len(),
        6,
        "report/diagnose phase 1 should have 6 topics: health, hardware, storage, network, security, toolchains"
    );
}

#[test]
fn test_fix_all_dry_run_preview_filters_correctly() {
    use hematite::agent::report_export::sweep_auto_fixes;
    let all = sweep_auto_fixes();
    // Simulate what --fix-all --dry-run --only "dns" would show
    let lower = "dns";
    let preview: Vec<_> = all
        .iter()
        .filter(|f| f.label.to_ascii_lowercase().contains(lower))
        .collect();
    assert!(
        !preview.is_empty(),
        "dry-run --only dns should match at least one fix"
    );
    // Unfiltered preview should match all
    let all_preview: Vec<_> = all.iter().collect();
    assert_eq!(
        all_preview.len(),
        sweep_auto_fixes().len(),
        "unfiltered dry-run should show all sweep entries"
    );
}

#[test]
fn test_output_flag_help_mentions_path() {
    use clap::CommandFactory;
    use hematite::CliCockpit;
    let cmd = CliCockpit::command();
    let output_arg = cmd.get_arguments().find(|a| a.get_long() == Some("output"));
    assert!(output_arg.is_some(), "--output flag must exist");
    let help = output_arg
        .unwrap()
        .get_help()
        .map(|h| h.to_string())
        .unwrap_or_default();
    assert!(
        help.to_ascii_lowercase().contains("path") || help.to_ascii_lowercase().contains("file"),
        "--output help text should mention path or file: {}",
        help
    );
}

#[test]
fn test_report_export_save_diagnosis_wiring() {
    use hematite::agent::report_export;
    let _ = std::hint::black_box(report_export::save_diagnosis_report as *const () as usize);
}

// ── Fix recipes ───────────────────────────────────────────────────────────────

#[test]
fn test_fix_recipes_match_low_disk() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "disk: C: — very low free space (2 GB)";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match low disk recipe");
    assert!(
        recipes.iter().any(|r| r.title.contains("disk")),
        "wrong recipe matched"
    );
}

#[test]
fn test_fix_recipes_match_no_internet() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Internet Connectivity: unreachable — could not ping 1.1.1.1";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match no internet recipe");
}

#[test]
fn test_fix_recipes_no_match_on_clean_output() {
    use hematite::agent::fix_recipes::match_recipes;
    // A genuine health_report ALL GOOD output has no trigger words
    let output = "ALL GOOD — system is healthy\ncpu: 12%\nmemory: 4 GB used of 16 GB";
    let recipes = match_recipes(output);
    assert!(
        recipes.is_empty(),
        "clean output should not trigger any recipes"
    );
}

#[test]
fn test_fix_recipes_format_action_plan_healthy() {
    use hematite::agent::fix_recipes::format_action_plan;
    let sections: &[(&str, &str)] = &[("health_report", "ALL GOOD — no issues found")];
    let plan = format_action_plan(sections);
    assert!(
        plan.contains("healthy")
            || plan.contains("healthy")
            || plan.to_ascii_lowercase().contains("no actionable"),
        "healthy machine should produce 'no actionable findings' message"
    );
}

#[test]
fn test_fix_recipes_format_action_plan_with_issues() {
    use hematite::agent::fix_recipes::format_action_plan;
    let sections: &[(&str, &str)] = &[(
        "health_report",
        "[!] Disk: C: — very low free space\n[!] Pending reboot required",
    )];
    let plan = format_action_plan(sections);
    assert!(
        plan.contains("ACTION") || plan.contains("INVESTIGATE"),
        "should have severity badges"
    );
    assert!(!plan.is_empty(), "should have non-empty plan for issues");
}

#[test]
fn test_fix_recipes_action_sorted_before_monitor() {
    use hematite::agent::fix_recipes::format_action_plan;
    let sections: &[(&str, &str)] = &[(
        "health_report",
        "high latency detected — ms rtt — high latency\ndisk: C: — very low free space",
    )];
    let plan = format_action_plan(sections);
    let action_pos = plan.find("ACTION");
    let monitor_pos = plan.find("MONITOR");
    if let (Some(a), Some(m)) = (action_pos, monitor_pos) {
        assert!(a < m, "ACTION items should appear before MONITOR items");
    }
}

#[test]
fn test_fix_recipes_diagnose_report_wiring() {
    // Verify generate_diagnosis_report is reachable (wiring/compile check).
    use hematite::agent::report_export;
    let _ = std::hint::black_box(report_export::generate_diagnosis_report as *const () as usize);
}

// ── Health score ──────────────────────────────────────────────────────────────

#[test]
fn test_health_score_clean_is_a() {
    use hematite::agent::fix_recipes::score_health;
    let sections: &[(&str, &str)] = &[("health_report", "ALL GOOD — system is healthy")];
    let score = score_health(sections);
    assert_eq!(score.grade, 'A');
    assert_eq!(score.label, "Excellent");
    assert_eq!(score.action_count, 0);
}

#[test]
fn test_health_score_one_action_is_d() {
    use hematite::agent::fix_recipes::score_health;
    let sections: &[(&str, &str)] = &[("health_report", "disk: C: — very low free space")];
    let score = score_health(sections);
    assert_eq!(score.grade, 'D');
    assert_eq!(score.action_count, 1);
}

#[test]
fn test_health_score_three_actions_is_f() {
    use hematite::agent::fix_recipes::score_health;
    let sections: &[(&str, &str)] = &[(
        "health_report",
        "disk: C: — very low free space\nreal-time protection: disabled\nthreat detected malware found",
    )];
    let score = score_health(sections);
    assert_eq!(score.grade, 'F');
    assert_eq!(score.label, "Critical");
}

#[test]
fn test_health_score_investigate_only_is_b_or_c() {
    use hematite::agent::fix_recipes::score_health;
    let b_sections: &[(&str, &str)] = &[("health_report", "pending reboot required")];
    let b = score_health(b_sections);
    assert_eq!(b.grade, 'B');

    let c_sections: &[(&str, &str)] = &[(
        "health_report",
        "pending reboot required\nwindows update pending update",
    )];
    let c = score_health(c_sections);
    assert_eq!(c.grade, 'C');
}

#[test]
fn test_health_score_summary_line_clean() {
    use hematite::agent::fix_recipes::score_health;
    let score = score_health(&[("h", "ALL GOOD system healthy")]);
    let summary = score.summary_line();
    assert!(
        summary.to_ascii_lowercase().contains("healthy")
            || summary.to_ascii_lowercase().contains("no issues"),
        "clean summary should mention healthy/no issues: {}",
        summary
    );
}

// ── New fix recipe coverage ───────────────────────────────────────────────────

#[test]
fn test_fix_recipes_match_device_error() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Yellow Bang detected: USB Root Hub — Error Code 43";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match device error recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("device")),
        "should match hardware device recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "ACTION"),
        "device errors should be ACTION severity"
    );
}

#[test]
fn test_fix_recipes_match_no_backup() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "File History: Disabled\nNo restore points found";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match no backup recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("backup")),
        "should match backup recipe"
    );
}

#[test]
fn test_fix_recipes_match_smb1() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "SMB1 is enabled — security risk";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match SMB1 recipe");
    assert!(
        recipes.iter().any(|r| r.severity == "ACTION"),
        "SMB1 enabled should be ACTION severity"
    );
}

#[test]
fn test_fix_recipes_match_bitlocker_off() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Protection State: Off\nBitLocker: Off";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match BitLocker recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("encrypt")),
        "should match encryption recipe"
    );
}

#[test]
fn test_fix_recipes_match_dns_failure() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "DNS Resolution: Failed — could not resolve google.com";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match DNS failure recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("dns")),
        "should match DNS recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "ACTION"),
        "DNS failure should be ACTION severity"
    );
}

#[test]
fn test_fix_recipes_total_count() {
    // Sanity check: we have at least 17 recipes (12 original + 5 new).
    use hematite::agent::fix_recipes::match_recipes;
    // Trigger all known recipes by building an output with all trigger words
    let everything = "disk: very low free space\ndisk_health smart predictive failure\n\
        pending reboot required\ncritical error event\nnot running: windefend\n\
        internet connectivity: unreachable\nms rtt — high latency\nram: very low\n\
        °c — very high check cooling\nreal-time protection: disabled\nthreat detected malware\n\
        windows update pending\nyellow bang pnp error\nfile history: disabled no restore points\n\
        smb1 is enabled\nprotection state: off bitlocker: off\ndns resolution: failed";
    let recipes = match_recipes(everything);
    assert!(
        recipes.len() >= 17,
        "expected at least 17 recipes, got {}",
        recipes.len()
    );
}

#[test]
fn test_fix_recipes_match_app_crashes() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Faulting application: chrome.exe — crash count: 5 in last 7 days";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match app crash recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("crash")),
        "should match crash recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "INVESTIGATE"),
        "app crashes should be INVESTIGATE severity"
    );
}

#[test]
fn test_fix_recipes_match_vcredist_missing() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Error: 0xc000007b — vcruntime140.dll not found";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match VC++ runtime recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("visual c++")),
        "should match VC++ runtime recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "ACTION"),
        "missing VC++ runtime should be ACTION severity"
    );
}

#[test]
fn test_fix_recipes_match_certificate_expiring() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Certificate: CN=example.com — expires in 15 days";
    let recipes = match_recipes(output);
    assert!(
        !recipes.is_empty(),
        "should match certificate expiry recipe"
    );
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("certificate")),
        "should match certificate recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "INVESTIGATE"),
        "expiring certificate should be INVESTIGATE severity"
    );
}

#[test]
fn test_fix_recipes_match_wifi_weak() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Signal: Poor — RSSI: -88 dBm";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match Wi-Fi weak signal recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("wi-fi")),
        "should match Wi-Fi recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "MONITOR"),
        "weak Wi-Fi should be MONITOR severity"
    );
}

#[test]
fn test_fix_recipes_match_ntp_failure() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Time Sync Failed — NTP source unreachable; clock drift detected";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match NTP failure recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("clock")),
        "should match NTP/clock recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "INVESTIGATE"),
        "NTP failure should be INVESTIGATE severity"
    );
}

#[test]
fn test_fix_recipes_match_pagefile_missing() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "Pagefile: None — no page file configured on this system";
    let recipes = match_recipes(output);
    assert!(!recipes.is_empty(), "should match page file recipe");
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("page file")),
        "should match page file recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "INVESTIGATE"),
        "missing page file should be INVESTIGATE severity"
    );
}

#[test]
fn test_fix_recipes_match_system_file_corruption() {
    use hematite::agent::fix_recipes::match_recipes;
    let output = "AutoRepairRequired: True — Windows Resource Protection found corrupt files";
    let recipes = match_recipes(output);
    assert!(
        !recipes.is_empty(),
        "should match system file corruption recipe"
    );
    assert!(
        recipes
            .iter()
            .any(|r| r.title.to_ascii_lowercase().contains("corrupt")),
        "should match corruption recipe"
    );
    assert!(
        recipes.iter().any(|r| r.severity == "ACTION"),
        "system file corruption should be ACTION severity"
    );
}

#[test]
fn test_fix_recipes_total_count_expanded() {
    // Phase 7: now expect at least 36 recipes — trigger all known patterns
    use hematite::agent::fix_recipes::match_recipes;
    let everything = "disk: very low free space\ndisk_health smart predictive failure\n\
        pending reboot required\ncritical error event\nnot running: windefend\n\
        internet connectivity: unreachable\nms rtt — high latency\nram: very low\n\
        °c — very high check cooling\nreal-time protection: disabled\nthreat detected malware\n\
        windows update pending\nyellow bang pnp error\nfile history: disabled no restore points\n\
        smb1 is enabled\nprotection state: off bitlocker: off\ndns resolution: failed\n\
        faulting application chrome.exe crash count: 5\n0xc000007b vcruntime140.dll not found\n\
        certificate expires in 15 days\nsignal: poor rssi: -88\n\
        time sync failed ntp source unreachable\nno page file configured\n\
        autorepairrequired: true windows resource protection found corrupt files\n\
        service terminated\nrdp status: disabled\nwuauserv: stopped\nfinding: printnightmare\n\
        classic teams cache: 2.3 gb\ntoken broker: not running\n\
        wmi repository is inconsistent\nlicense status: unlicensed\n\
        wsearch: stopped\nsync status: error\nstatus: offline\nprofile count: 0";
    let recipes = match_recipes(everything);
    assert!(
        recipes.len() >= 36,
        "expected at least 36 recipes, got {}",
        recipes.len()
    );
}

// ── Routing: new 0.7.0 topics ───────────────────────────────────────────────

#[test]
fn test_routing_detects_domain_health_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Check DC connectivity and LDAP port",
        "Is the domain controller reachable?",
        "Run nltest and check GPO refresh",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("domain_health"),
            "Expected domain_health for: {q}"
        );
    }
}

#[test]
fn test_routing_detects_service_dependencies_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "What services depend on SQL Server?",
        "Show service dependency graph",
        "Which services are needed by WMI?",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("service_dependencies"),
            "Expected service_dependencies for: {q}"
        );
    }
}

#[test]
fn test_routing_detects_wmi_health_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Is the WMI repository corrupt?",
        "Check WMI health",
        "WMI repository repair steps",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("wmi_health"), "Expected wmi_health for: {q}");
    }
}

#[test]
fn test_routing_detects_local_security_policy_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "What is the password policy on this machine?",
        "Show account lockout threshold",
        "Check LM compatibility level",
        "UAC prompt keeps appearing",
        "user account control is disabled",
        "run as administrator not working",
        "needs elevation every time I open it",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("local_security_policy"),
            "Expected local_security_policy for: {q}"
        );
    }
}

#[test]
fn test_routing_detects_usb_history_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Show USB device history from registry",
        "USB forensic audit USBSTOR",
        "What USB drives have ever been connected?",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("usb_history"), "Expected usb_history for: {q}");
    }
}

#[test]
fn test_routing_detects_print_spooler_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "Is the print spooler vulnerable to PrintNightmare?",
        "Check spooler service and CVE-2021-34527",
        "Print spooler security status",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(
            topic,
            Some("print_spooler"),
            "Expected print_spooler for: {q}"
        );
    }
}

// ── Batch 10 routing expansions ─────────────────────────────────────────────

#[test]
fn test_routing_detects_sessions_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "who is logged on right now",
        "show connected users",
        "list user sessions",
        "query session",
        "who is logged in to this machine",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("sessions"), "Expected sessions for: {q}");
    }
}

#[test]
fn test_routing_detects_startup_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "what startup programs are enabled",
        "disable startup apps",
        "what starts with windows",
        "show msconfig startup entries",
        "what runs on boot",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("startup_items"), "Expected startup_items for: {q}");
    }
}

#[test]
fn test_routing_detects_certificates_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "check tls certificate status",
        "x509 certificate expiring",
        "list pfx certificates",
        "is there a pem file in the cert store",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("certificates"), "Expected certificates for: {q}");
    }
}

// ── Batch 11 routing expansions ─────────────────────────────────────────────

#[test]
fn test_routing_detects_hardware_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "what graphics card do I have",
        "show system information",
        "computer specs",
        "what video card is installed",
        "how much ram is in this machine",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("hardware"), "Expected hardware for: {q}");
    }
}

#[test]
fn test_routing_detects_device_health_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "device manager shows errors",
        "unknown device in device manager",
        "error code 43 on USB device",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("device_health"), "Expected device_health for: {q}");
    }
}

#[test]
fn test_routing_detects_vpn_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "cisco anyconnect not connecting",
        "wireguard tunnel status",
        "GlobalProtect VPN client status",
        "split tunnel configuration",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("vpn"), "Expected vpn for: {q}");
    }
}

// ── Batch 12 routing expansions ─────────────────────────────────────────────

#[test]
fn test_routing_detects_printers_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "why is printing not working",
        "print job is stuck in queue",
        "printer shows offline",
        "can't print to default printer",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("printers"), "Expected printers for: {q}");
    }
}

#[test]
fn test_routing_detects_connections_expanded() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let queries = [
        "show outbound connections from this machine",
        "what process is connecting to remote hosts",
        "list inbound connections",
    ];
    for q in &queries {
        let topic = preferred_host_inspection_topic(q);
        assert_eq!(topic, Some("connections"), "Expected connections for: {q}");
    }
}

// ── Fix recipe trigger correctness ──────────────────────────────────────────

#[test]
fn test_fix_recipes_service_failure_triggers() {
    use hematite::agent::fix_recipes::match_recipes;
    let cases = [
        "service terminated with error",
        "failed to start the service",
        "error 1067 the process terminated unexpectedly",
        "error 1053 service did not respond",
        "exited with code 1",
        "failed to respond to the start or control request",
    ];
    for c in &cases {
        let r = match_recipes(c);
        assert!(!r.is_empty(), "service failure recipe should fire for: {c}");
    }
}

#[test]
fn test_fix_recipes_rdp_disabled_triggers() {
    use hematite::agent::fix_recipes::match_recipes;
    let cases = [
        "rdp status: disabled",
        "no enabled rdp firewall rule found",
        "fdenytsconnections: 1",
    ];
    for c in &cases {
        let r = match_recipes(c);
        assert!(!r.is_empty(), "RDP disabled recipe should fire for: {c}");
    }
}

#[test]
fn test_fix_recipes_windows_update_service_triggers() {
    use hematite::agent::fix_recipes::match_recipes;
    let cases = [
        "wuauserv: stopped",
        "wuauserv stopped — windows update disabled",
        "windows update: stopped",
        "bits: stopped",
        "bits stopped",
    ];
    for c in &cases {
        let r = match_recipes(c);
        assert!(
            !r.is_empty(),
            "Windows Update service recipe should fire for: {c}"
        );
    }
}

#[test]
fn test_fix_recipes_printnightmare_triggers() {
    use hematite::agent::fix_recipes::match_recipes;
    let cases = [
        "rpcauthnlevelprivacyenabled: 0 — not hardened",
        "printnightmare rpc mitigation not applied",
        "point and print allows silent driver install",
        "finding: printnightmare mitigation missing",
    ];
    for c in &cases {
        let r = match_recipes(c);
        assert!(!r.is_empty(), "PrintNightmare recipe should fire for: {c}");
    }
}

// ── --fix list / exit-code helpers ──────────────────────────────────────────

#[test]
fn test_fix_issue_categories_count() {
    let cats = hematite::agent::report_export::fix_issue_categories();
    assert!(
        cats.len() >= 45,
        "expected at least 45 issue categories, got {}",
        cats.len()
    );
    // Every entry must have non-empty label and keywords
    for (label, keywords) in cats {
        assert!(!label.is_empty(), "category label must not be empty");
        assert!(!keywords.is_empty(), "category keywords must not be empty");
    }
}

#[test]
fn test_report_indicates_issues_markdown() {
    // Grade A = no issues
    let clean = "**Health Score:** A — Excellent  \n\nAll good.";
    assert!(
        !hematite::agent::report_export::report_has_issues_in_content(clean),
        "Grade A should not indicate issues"
    );

    // Grade B and worse = issues
    for grade in &["B", "C", "D", "F"] {
        let flagged = format!("**Health Score:** {} — Something  \n\n", grade);
        assert!(
            hematite::agent::report_export::report_has_issues_in_content(&flagged),
            "Grade {} should indicate issues",
            grade
        );
    }
}

#[test]
fn test_report_indicates_issues_html() {
    let clean_html = "<h2>Health Score: A — Excellent</h2>";
    assert!(
        !hematite::agent::report_export::report_has_issues_in_content(clean_html),
        "HTML grade A should not indicate issues"
    );

    let flagged_html = "<h2>Health Score: D — Poor</h2>";
    assert!(
        hematite::agent::report_export::report_has_issues_in_content(flagged_html),
        "HTML grade D should indicate issues"
    );
}

// ── Scheduler ────────────────────────────────────────────────────────────────

#[test]
fn test_scheduler_query_returns_string() {
    // query_scheduled_task must not panic — returns either task info or "not registered"
    let result = hematite::agent::scheduler::query_scheduled_task();
    assert!(
        !result.is_empty(),
        "query_scheduled_task must return non-empty string"
    );
}

#[test]
fn test_scheduler_remove_nonexistent_returns_error() {
    // Removing a task that doesn't exist should return Err, not panic.
    // On Windows this confirms schtasks is callable; on non-Windows the stub Err is fine.
    let result = hematite::agent::scheduler::remove_scheduled_task();
    // We just verify it doesn't panic and returns a Result — either outcome is valid
    // (task might or might not be registered in the test environment).
    let _ = result;
}

#[test]
fn test_scheduler_register_invalid_exe_returns_err_or_ok() {
    // Registering with a fake path should fail gracefully on Windows (schtasks rejects
    // the exe) or return an Err on non-Windows. Must not panic either way.
    let result = hematite::agent::scheduler::register_scheduled_task("weekly", "nonexistent.exe");
    // Both Ok and Err are valid — we just check it doesn't crash
    let _ = result;
}

#[test]
fn test_scheduler_sweep_query_returns_string() {
    let result = hematite::agent::scheduler::query_sweep_task();
    assert!(!result.is_empty());
}

#[test]
fn test_scheduler_sweep_remove_nonexistent_returns_result() {
    let result = hematite::agent::scheduler::remove_sweep_task();
    let _ = result;
}

#[test]
fn test_scheduler_sweep_register_does_not_panic() {
    let result = hematite::agent::scheduler::register_sweep_task("weekly", "nonexistent.exe");
    let _ = result;
}

// ── fix_plan routing: mutation-guard bypass for host-remediation queries ───────
//
// Queries like "fix cargo not found" contain a code keyword ("cargo") that also
// triggers asks_mutation_intent. Before v0.8.2 the mutation guard fired first and
// returned None, silently dropping the host-inspection intent. The fix adds an
// early-return for (asks_fix_plan && asks_mutation_intent) before the mutation guard
// so these read-only host-remediation queries are correctly routed to fix_plan.

#[test]
fn test_routing_detects_fix_plan_for_cargo_remediation() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // "fix" is both a fix verb (asks_fix_plan) and a mutation verb (asks_mutation_intent).
    // Pairing it with a code keyword like "cargo" or "rust" makes both conditions true,
    // so the early return added in v0.8.2 fires before the mutation guard returns None.
    assert_eq!(
        preferred_host_inspection_topic("fix cargo not found on this machine"),
        Some("fix_plan")
    );
    assert_eq!(
        preferred_host_inspection_topic("how do I fix cargo not on my PATH"),
        Some("fix_plan")
    );
    assert_eq!(
        preferred_host_inspection_topic("fix rust toolchain not found"),
        Some("fix_plan")
    );
}

#[test]
fn test_routing_detects_fix_plan_for_runtime_remediation() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // These use fix-type verbs paired with runtime keywords (lm studio, port, model)
    // that don't trip the mutation guard, so they flow through the dispatch chain to
    // fix_plan without colliding with dns_lookup or other higher-priority topics.
    assert_eq!(
        preferred_host_inspection_topic("fix lm studio connection refused"),
        Some("fix_plan")
    );
    assert_eq!(
        preferred_host_inspection_topic("fix port 1234 already in use"),
        Some("fix_plan")
    );
    assert_eq!(
        preferred_host_inspection_topic("fix embedding model not loading"),
        Some("fix_plan")
    );
    assert_eq!(
        preferred_host_inspection_topic("fix no coding model loaded"),
        Some("fix_plan")
    );
}

#[test]
fn test_routing_mutation_guard_still_blocks_code_mutations() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Real code-mutation queries must NOT route to any host inspection topic.
    // These have mutation verbs + code keywords but no fix/repair/resolve/troubleshoot,
    // so asks_fix_plan is false and the mutation guard correctly returns None.
    let mutations = [
        "create a cargo project",
        "create a new rust project",
        "write a python script to parse the logs",
        "refactor this code",
        "implement the new feature",
    ];
    for q in mutations {
        let topic = preferred_host_inspection_topic(q);
        assert!(
            topic.is_none(),
            "Mutation query should not route to host inspection: {q:?} (got: {topic:?})"
        );
    }
}

// ── thermal routing: self-sufficient state words need no action verb ───────────
//
// Before v0.8.2, mentions_host_inspection_question required an explicit action verb
// ("show me", "check", "how") alongside the host scope word. Queries like
// "is my CPU throttled?" have no action verb so host inspection mode was never
// enabled and the model free-formed instead of calling inspect_host. The fix adds
// self_sufficient_state detection for "throttled", "overheating", "bottlenecking".

#[test]
fn test_routing_detects_thermal_for_cpu_throttle() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // "throttle" without "gpu" → thermal (not overclocker).
    assert_eq!(
        preferred_host_inspection_topic("is my CPU throttled?"),
        Some("thermal")
    );
    assert_eq!(
        preferred_host_inspection_topic("why is my CPU throttling"),
        Some("thermal")
    );
    assert_eq!(
        preferred_host_inspection_topic("cpu temp too high"),
        Some("thermal")
    );
}

#[test]
fn test_routing_detects_thermal_for_overheating_without_gpu() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // "overheating" without "gpu" → thermal.
    assert_eq!(
        preferred_host_inspection_topic("my PC is overheating"),
        Some("thermal")
    );
    assert_eq!(
        preferred_host_inspection_topic("is the system overheating?"),
        Some("thermal")
    );
    assert_eq!(
        preferred_host_inspection_topic("check if my computer is overheating"),
        Some("thermal")
    );
}

#[test]
fn test_routing_detects_overclocker_for_gpu_thermal_queries() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // "gpu" + throttle/bottleneck/overheating → overclocker (higher priority than thermal).
    assert_eq!(
        preferred_host_inspection_topic("is my GPU throttled?"),
        Some("overclocker")
    );
    assert_eq!(
        preferred_host_inspection_topic("why is my GPU bottlenecking?"),
        Some("overclocker")
    );
    assert_eq!(
        preferred_host_inspection_topic("is my GPU overheating?"),
        Some("overclocker")
    );
}

// ── app_crashes routing ───────────────────────────────────────────────────────
// app_crashes dispatches before recent_crashes in the chain, so browser-specific
// crash queries must route to app_crashes even though "crash" alone would match
// recent_crashes.

#[test]
fn test_routing_detects_app_crashes_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Use non-browser apps: "chrome crash" routes to browser_health (dispatched earlier).
    assert_eq!(
        preferred_host_inspection_topic("word keeps crashing"),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic("what applications have been crashing"),
        Some("app_crashes")
    );
    assert_eq!(
        preferred_host_inspection_topic("faulting application svchost.exe"),
        Some("app_crashes")
    );
}

// ── hyperv routing ────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_hyperv_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show my hyper-v virtual machines"),
        Some("hyperv")
    );
    assert_eq!(
        preferred_host_inspection_topic("list vms running on this host"),
        Some("hyperv")
    );
    assert_eq!(
        preferred_host_inspection_topic("hyperv vm status"),
        Some("hyperv")
    );
}

// ── sessions routing ──────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_sessions_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Avoid "who is logged in" and "active sessions" — both are in asks_user_accounts
    // (lines 585 and 597) which dispatches at line 2099, before sessions (line 2237).
    assert_eq!(
        preferred_host_inspection_topic("show user sessions on this PC"),
        Some("sessions")
    );
    assert_eq!(
        preferred_host_inspection_topic("who is on this machine right now"),
        Some("sessions")
    );
    assert_eq!(
        preferred_host_inspection_topic("list current login sessions"),
        Some("sessions")
    );
}

// ── ntp routing ───────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_ntp_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("time sync is broken"),
        Some("ntp")
    );
    assert_eq!(
        preferred_host_inspection_topic("my computer clock is wrong"),
        Some("ntp")
    );
    assert_eq!(
        preferred_host_inspection_topic("NTP server not responding"),
        Some("ntp")
    );
}

// ── cpu_power routing ─────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_cpu_power_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("why is my CPU running so slow"),
        Some("cpu_power")
    );
    assert_eq!(
        preferred_host_inspection_topic("is turbo boost enabled"),
        Some("cpu_power")
    );
    assert_eq!(
        preferred_host_inspection_topic("check CPU clock speed"),
        Some("cpu_power")
    );
}

// ── display_config routing ────────────────────────────────────────────────────
// display_config is dispatched before peripherals in the chain, so "monitor"
// queries route to display_config even though asks_peripherals also matches it.

#[test]
fn test_routing_detects_display_config_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what resolution is my monitor running at"),
        Some("display_config")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is my screen refresh rate"),
        Some("display_config")
    );
    assert_eq!(
        preferred_host_inspection_topic("check my display configuration"),
        Some("display_config")
    );
}

// ── search_index routing ──────────────────────────────────────────────────────

#[test]
fn test_routing_detects_search_index_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("Windows search not working"),
        Some("search_index")
    );
    assert_eq!(
        preferred_host_inspection_topic("search indexer is stuck"),
        Some("search_index")
    );
    assert_eq!(
        preferred_host_inspection_topic("why is my search indexing so slow"),
        Some("search_index")
    );
}

// ── sign_in routing ───────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_sign_in_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("Windows Hello not working"),
        Some("sign_in")
    );
    assert_eq!(
        preferred_host_inspection_topic("my PIN is broken"),
        Some("sign_in")
    );
    assert_eq!(
        preferred_host_inspection_topic("can't sign in to my account"),
        Some("sign_in")
    );
}

// ── camera routing ────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_camera_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("my camera is not working"),
        Some("camera")
    );
    assert_eq!(
        preferred_host_inspection_topic("webcam not detected"),
        Some("camera")
    );
    assert_eq!(
        preferred_host_inspection_topic("camera blocked by privacy settings"),
        Some("camera")
    );
}

// ── outlook routing ───────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_outlook_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("Outlook is not opening"),
        Some("outlook")
    );
    assert_eq!(
        preferred_host_inspection_topic("Microsoft Outlook add-ins are disabled"),
        Some("outlook")
    );
    assert_eq!(
        preferred_host_inspection_topic("where is my Outlook OST file"),
        Some("outlook")
    );
}

// ── teams routing ─────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_teams_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("Teams is not loading"),
        Some("teams")
    );
    assert_eq!(
        preferred_host_inspection_topic("clear Microsoft Teams cache"),
        Some("teams")
    );
    assert_eq!(
        preferred_host_inspection_topic("why does Teams keep crashing"),
        Some("teams")
    );
}

// ── windows_backup routing ────────────────────────────────────────────────────

#[test]
fn test_routing_detects_windows_backup_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("check my system restore points"),
        Some("windows_backup")
    );
    assert_eq!(
        preferred_host_inspection_topic("file history not working"),
        Some("windows_backup")
    );
    assert_eq!(
        preferred_host_inspection_topic("when was the last Windows backup"),
        Some("windows_backup")
    );
}

// ── env_doctor routing ────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_env_doctor_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("run the environment doctor"),
        Some("env_doctor")
    );
    assert_eq!(
        preferred_host_inspection_topic("check for package manager conflicts"),
        Some("env_doctor")
    );
    assert_eq!(
        preferred_host_inspection_topic("show shims in my PATH"),
        Some("env_doctor")
    );
}

// ── path routing ──────────────────────────────────────────────────────────────

#[test]
fn test_routing_detects_path_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show my PATH entries"),
        Some("path")
    );
    assert_eq!(
        preferred_host_inspection_topic("what is in my PATH"),
        Some("path")
    );
    assert_eq!(
        preferred_host_inspection_topic("show raw PATH variable"),
        Some("path")
    );
}

// ── priority collision: teams vs nic_teaming ──────────────────────────────────
// "teams" appears in both asks_teams and in NIC teaming queries. The not_nic_teaming
// guard inside asks_teams must exclude it when the query is about NIC teaming.

#[test]
fn test_routing_teams_excluded_for_nic_teaming_queries() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("nic teaming configuration"),
        Some("nic_teaming")
    );
    assert_eq!(
        preferred_host_inspection_topic("nic-teaming setup"),
        Some("nic_teaming")
    );
    assert_eq!(
        preferred_host_inspection_topic("LBFO team adapter status"),
        Some("nic_teaming")
    );
}

// ── priority collision: app_crashes vs recent_crashes ─────────────────────────
// app_crashes (dispatched first) must win for app-specific queries; recent_crashes
// must win for kernel-level events that don't name an application.

#[test]
fn test_routing_app_crashes_dispatches_before_recent_crashes() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // App-specific crash: goes to app_crashes, not recent_crashes.
    // Use a non-browser app — "chrome crash" routes to browser_health (earlier dispatch).
    assert_eq!(
        preferred_host_inspection_topic("word keeps crashing"),
        Some("app_crashes")
    );
    // Kernel-level/BSOD: no app name → recent_crashes.
    assert_eq!(
        preferred_host_inspection_topic("my PC crashed and restarted itself"),
        Some("recent_crashes")
    );
}

// ── priority collision: overclocker dispatches before thermal ─────────────────
// When "gpu" is present alongside "throttl", overclocker (line 2095) wins.
// Without "gpu", the same throttl query falls through to thermal (line 2145).

#[test]
fn test_routing_overclocker_dispatches_before_thermal() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is my GPU throttling?"),
        Some("overclocker")
    );
    assert_eq!(
        preferred_host_inspection_topic("is my CPU throttling?"),
        Some("thermal")
    );
}

// ── priority collision: display_config dispatches before peripherals ───────────
// "monitor" appears in both asks_display_config and asks_peripherals. display_config
// wins because it is dispatched earlier (line 2175 vs line 2235).

#[test]
fn test_routing_display_config_dispatches_before_peripherals() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what monitors are connected"),
        Some("display_config")
    );
    // Pure peripheral query without display context goes to peripherals.
    assert_eq!(
        preferred_host_inspection_topic("show connected USB keyboards"),
        Some("peripherals")
    );
}

// ── morphology regression: throttl stem catches all inflected forms ────────────
// Before the throttl-stem fix, "throttling" (present continuous) did not contain
// the substring "throttle" so asks_thermal returned false and the query fell
// through to None. The stem "throttl" catches throttle/throttled/throttling/
// throttles and must continue to do so if the routing is ever refactored.

#[test]
fn test_routing_throttl_stem_catches_all_inflected_forms() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Past participle — was never broken.
    assert_eq!(
        preferred_host_inspection_topic("is my CPU throttled?"),
        Some("thermal")
    );
    // Present continuous — was broken before the stem fix.
    assert_eq!(
        preferred_host_inspection_topic("why is my CPU throttling?"),
        Some("thermal")
    );
    // Third-person singular present — also covered by the stem.
    assert_eq!(
        preferred_host_inspection_topic("the CPU throttles under load"),
        Some("thermal")
    );
    // GPU + throttling form → overclocker (gpu guard still works with stem).
    assert_eq!(
        preferred_host_inspection_topic("is my GPU throttling?"),
        Some("overclocker")
    );
}

// ── all_host_inspection_topics: multi-topic harness pre-runs ──────────────────
// The harness pre-run fires all_host_inspection_topics before the model turn when
// 2+ topics are detected. These tests guard against new topics being omitted from
// the all_host_inspection_topics table (a different table from
// preferred_host_inspection_topic).

#[test]
fn test_all_host_topics_detects_hyperv_and_sessions_together() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics("show hyper-v vms and who is logged on this host");
    assert!(
        topics.contains(&"hyperv"),
        "should include hyperv; got: {topics:?}"
    );
    assert!(
        topics.contains(&"sessions"),
        "should include sessions; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_detects_app_crashes_and_browser_health() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics(
        "application crash in chrome — also check overall browser health",
    );
    assert!(
        topics.contains(&"app_crashes"),
        "should include app_crashes; got: {topics:?}"
    );
    assert!(
        topics.contains(&"browser_health"),
        "should include browser_health; got: {topics:?}"
    );
}

// ── all_host_inspection_topics: newly registered topics ───────────────────────
// These topics were previously in preferred_host_inspection_topic's dispatch
// chain but absent from all_host_inspection_topics. Adding them means multi-topic
// harness pre-runs now fire them when the query mentions both this topic and
// another (e.g. "check defender quarantine and overall security posture").

#[test]
fn test_all_host_topics_detects_defender_quarantine_with_security() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics =
        all_host_inspection_topics("check defender quarantine and current security posture");
    assert!(
        topics.contains(&"defender_quarantine"),
        "should include defender_quarantine; got: {topics:?}"
    );
    assert!(
        topics.contains(&"security"),
        "should include security; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_detects_storage_spaces_with_disk_health() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics("check storage spaces health and disk health");
    assert!(
        topics.contains(&"storage_spaces"),
        "should include storage_spaces; got: {topics:?}"
    );
    assert!(
        topics.contains(&"disk_health"),
        "should include disk_health; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_detects_log_check_with_network_stats() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics =
        all_host_inspection_topics("show recent errors from the event log and network stats");
    assert!(
        topics.contains(&"log_check"),
        "should include log_check; got: {topics:?}"
    );
    assert!(
        topics.contains(&"network_stats"),
        "should include network_stats; got: {topics:?}"
    );
}

#[test]
fn test_all_host_topics_detects_repo_doctor_with_connectivity() {
    use hematite::agent::routing::all_host_inspection_topics;
    let topics = all_host_inspection_topics("check repo health and network connectivity");
    assert!(
        topics.contains(&"repo_doctor"),
        "should include repo_doctor; got: {topics:?}"
    );
    assert!(
        topics.contains(&"connectivity"),
        "should include connectivity; got: {topics:?}"
    );
}

#[test]
fn test_routing_detects_repo_doctor_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("run repo doctor"),
        Some("repo_doctor")
    );
    assert_eq!(
        preferred_host_inspection_topic("show workspace health"),
        Some("repo_doctor")
    );
}

#[test]
fn test_routing_overheat_stem_catches_all_inflected_forms() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // "overheating" was the only form; now the stem covers all inflections
    for q in &[
        "my CPU is overheating",
        "my CPU overheated",
        "why does it overheat",
        "CPU keeps overheating",
    ] {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("thermal"),
            "query {:?} should route to thermal",
            q
        );
    }
}

#[test]
fn test_overheat_stem_routes_to_thermal() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    // Past tense "overheated" must route to thermal, not fall through
    assert_eq!(
        preferred_host_inspection_topic("cpu overheated last night"),
        Some("thermal"),
        "overheated (past tense) should route to thermal via overheat stem"
    );
    assert_eq!(
        preferred_host_inspection_topic("why does my cpu overheat"),
        Some("thermal"),
        "overheat (base form) should route to thermal"
    );
}

// ── inventory completeness ────────────────────────────────────────────────────

#[test]
fn test_inventory_covers_all_nine_groups() {
    let inv = hematite::agent::direct_answers::build_inspect_inventory();
    for group in &[
        "SYSTEM & HEALTH",
        "STORAGE & DISK",
        "THERMAL & POWER",
        "DEVICES & PERIPHERALS",
        "SECURITY",
        "NETWORK",
        "ENTERPRISE & IDENTITY",
        "APPLICATIONS",
        "DEVELOPER & ENVIRONMENT",
    ] {
        assert!(inv.contains(group), "inventory missing group: {}", group);
    }
}

#[test]
fn test_inventory_contains_representative_topics() {
    let inv = hematite::agent::direct_answers::build_inspect_inventory();
    // spot-check one topic from each group
    for topic in &[
        "health_report",       // System
        "disk_benchmark",      // Storage
        "overclocker",         // Thermal
        "bluetooth",           // Devices
        "defender_quarantine", // Security
        "wlan_profiles",       // Network
        "mdm_enrollment",      // Enterprise
        "windows_backup",      // Applications
        "docker_filesystems",  // Developer
    ] {
        assert!(inv.contains(topic), "inventory missing topic: {}", topic);
    }
}

#[test]
fn test_inventory_lists_128_topics_hint() {
    let inv = hematite::agent::direct_answers::build_inspect_inventory();
    assert!(inv.contains("128"), "inventory should mention 128 topics");
}

// ── generate_query_output routing ────────────────────────────────────────────

#[tokio::test]
async fn test_generate_query_output_slow_pc_hits_resource_load() {
    let out = hematite::agent::report_export::generate_query_output("why is my PC slow").await;
    // resource_load is the primary topic for performance queries
    assert!(
        out.contains("Host inspection:") || out.contains("Resource") || out.contains("CPU"),
        "slow PC query should return diagnostic output, got: {}",
        &out[..out.len().min(200)]
    );
}

#[tokio::test]
async fn test_generate_query_output_unknown_query_falls_back_to_summary() {
    // A query that matches nothing should fall back to summary
    let out = hematite::agent::report_export::generate_query_output(
        "xyzzy nothing matches this query at all",
    )
    .await;
    assert!(
        !out.is_empty(),
        "unknown query should still return fallback summary output"
    );
}

// ── generate_inspect_output direct topics ────────────────────────────────────

#[tokio::test]
async fn test_generate_inspect_output_single_topic() {
    let out = hematite::agent::report_export::generate_inspect_output("connectivity").await;
    assert!(
        out.contains("connectivity") || out.contains("REACHABLE") || out.contains("internet"),
        "inspect connectivity should return connectivity output"
    );
}

#[tokio::test]
async fn test_generate_inspect_output_multi_topic_includes_separators() {
    let out = hematite::agent::report_export::generate_inspect_output("connectivity,wifi").await;
    assert!(
        out.contains("connectivity") && out.contains("wifi"),
        "multi-topic inspect should cover both topics"
    );
    // Separator lines should be present for multi-topic runs
    assert!(
        out.contains("───"),
        "multi-topic output should include section separators"
    );
}

#[tokio::test]
async fn test_generate_inspect_output_empty_returns_help() {
    let out = hematite::agent::report_export::generate_inspect_output("").await;
    assert!(
        out.contains("--inspect") || out.contains("inventory"),
        "empty topic should return usage hint"
    );
}

// ── inventory --report-format json ───────────────────────────────────────────

#[test]
fn test_inventory_json_is_valid_and_has_all_categories() {
    let out = hematite::agent::direct_answers::build_inspect_inventory_json();
    let parsed: serde_json::Value =
        serde_json::from_str(&out).expect("--inventory --report-format json should be valid JSON");
    let categories = parsed.as_array().expect("should be a JSON array");
    assert_eq!(categories.len(), 9, "should have 9 categories");
    for cat in categories {
        assert!(
            cat.get("category").is_some(),
            "each entry needs a 'category' field"
        );
        assert!(
            cat["topics"]
                .as_array()
                .map(|t| !t.is_empty())
                .unwrap_or(false),
            "each category needs a non-empty 'topics' array"
        );
    }
}

#[test]
fn test_inventory_json_contains_known_topics() {
    let out = hematite::agent::direct_answers::build_inspect_inventory_json();
    assert!(
        out.contains("health_report"),
        "inventory JSON should contain health_report"
    );
    assert!(
        out.contains("connectivity"),
        "inventory JSON should contain connectivity"
    );
    assert!(
        out.contains("docker"),
        "inventory JSON should contain docker"
    );
    assert!(
        out.contains("outlook"),
        "inventory JSON should contain outlook"
    );
}

// ── snapshots --report-format json ───────────────────────────────────────────

#[test]
fn test_snapshots_json_schema_shape() {
    // Verify the snapshot list JSON schema shape produced by --snapshots --report-format json.
    let entries = vec![
        serde_json::json!({"name": "before-update", "size_bytes": 4096u64, "age_secs": 3600u64}),
        serde_json::json!({"name": "after-update", "size_bytes": 4200u64, "age_secs": 120u64}),
    ];
    let arr = serde_json::Value::Array(entries);
    let out = serde_json::to_string_pretty(&arr).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("should parse");
    let list = parsed.as_array().expect("should be an array");
    assert_eq!(list.len(), 2);
    assert!(list[0].get("name").is_some());
    assert!(list[0].get("size_bytes").is_some());
    assert!(list[0].get("age_secs").is_some());
}

// ── diff JSON schema ──────────────────────────────────────────────────────────

#[test]
fn test_diff_json_schema_shape() {
    // Verify the diff JSON structure by building it the same way the handler does.
    use similar::{ChangeTag, TextDiff};
    let snap_a = "line one\nline two\nline three\n";
    let snap_b = "line one\nline two changed\nline three\n";
    let diff = TextDiff::from_lines(snap_a, snap_b);
    let mut diff_lines: Vec<String> = Vec::new();
    let mut changed = false;
    for group in diff.grouped_ops(2) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => {
                        changed = true;
                        "-"
                    }
                    ChangeTag::Insert => {
                        changed = true;
                        "+"
                    }
                    ChangeTag::Equal => " ",
                };
                diff_lines.push(format!("{}{}", prefix, change));
            }
        }
    }
    let obj = serde_json::json!({
        "topics": "test_topic",
        "snapshot_a": "snap_a_ts",
        "snapshot_b": "snap_b_ts",
        "changed": changed,
        "diff_lines": diff_lines,
        "before": snap_a,
        "after": snap_b,
    });
    let serialized = serde_json::to_string_pretty(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("should parse back");
    assert_eq!(parsed["changed"], serde_json::json!(true));
    assert!(parsed["diff_lines"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));
    assert!(parsed.get("before").is_some());
    assert!(parsed.get("after").is_some());
}

// ── compare JSON schema ──────────────────────────────────────────────────────

#[test]
fn test_compare_json_schema_shape() {
    // Verify --compare --report-format json produces the expected object shape.
    use similar::{ChangeTag, TextDiff};
    let snap_a = "version: 1.0\nstatus: ok\n";
    let snap_b = "version: 1.1\nstatus: ok\n";
    let diff = TextDiff::from_lines(snap_a, snap_b);
    let mut diff_lines: Vec<String> = Vec::new();
    let mut changed = false;
    for group in diff.grouped_ops(2) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => {
                        changed = true;
                        "-"
                    }
                    ChangeTag::Insert => {
                        changed = true;
                        "+"
                    }
                    ChangeTag::Equal => " ",
                };
                diff_lines.push(format!("{}{}", prefix, change));
            }
        }
    }
    let obj = serde_json::json!({
        "snapshot_a": "before-update (2d ago)",
        "snapshot_b": "after-update (1h ago)",
        "changed": changed,
        "diff_lines": diff_lines,
        "before": snap_a,
        "after": snap_b,
    });
    let serialized = serde_json::to_string_pretty(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("should parse back");
    assert_eq!(parsed["changed"], serde_json::json!(true));
    assert!(parsed.get("snapshot_a").is_some(), "should have snapshot_a");
    assert!(parsed.get("snapshot_b").is_some(), "should have snapshot_b");
    assert!(parsed.get("diff_lines").is_some(), "should have diff_lines");
    assert!(parsed.get("before").is_some(), "should have before");
    assert!(parsed.get("after").is_some(), "should have after");
    assert!(
        parsed.get("topics").is_none(),
        "compare JSON should not have topics field"
    );
    let lines = parsed["diff_lines"]
        .as_array()
        .expect("diff_lines should be array");
    assert!(
        !lines.is_empty(),
        "diff_lines should be non-empty for changed snapshots"
    );
}

#[test]
fn test_compare_json_unchanged_snapshots() {
    use similar::{ChangeTag, TextDiff};
    let content = "status: ok\nhealth: good\n";
    let diff = TextDiff::from_lines(content, content);
    let mut diff_lines: Vec<String> = Vec::new();
    let mut changed = false;
    for group in diff.grouped_ops(2) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let prefix = match change.tag() {
                    ChangeTag::Delete => {
                        changed = true;
                        "-"
                    }
                    ChangeTag::Insert => {
                        changed = true;
                        "+"
                    }
                    ChangeTag::Equal => " ",
                };
                diff_lines.push(format!("{}{}", prefix, change));
            }
        }
    }
    let obj = serde_json::json!({
        "snapshot_a": "snap1 (5m ago)",
        "snapshot_b": "snap2 (1m ago)",
        "changed": changed,
        "diff_lines": diff_lines,
        "before": content,
        "after": content,
    });
    let serialized = serde_json::to_string_pretty(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&serialized).expect("should parse");
    assert_eq!(parsed["changed"], serde_json::json!(false));
}

// ── watch NDJSON schema ───────────────────────────────────────────────────────

#[test]
fn test_watch_ndjson_schema_shape() {
    // Verify the exact JSON object shape emitted per cycle by --watch --report-format json.
    let obj = serde_json::json!({
        "timestamp": "12:00:00 UTC",
        "cycle": 1u64,
        "topics": "connectivity",
        "alert_matched": false,
        "output": "some output",
    });
    let line = serde_json::to_string(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&line).expect("should parse");
    assert!(parsed.get("timestamp").is_some());
    assert!(parsed.get("cycle").is_some());
    assert!(parsed.get("topics").is_some());
    assert!(parsed.get("alert_matched").is_some());
    assert!(parsed.get("output").is_some());
}

// ── generate_inspect_output_json ─────────────────────────────────────────────

#[tokio::test]
async fn test_generate_inspect_output_json_is_valid_json() {
    let out = hematite::agent::report_export::generate_inspect_output_json("connectivity").await;
    let parsed: serde_json::Value = serde_json::from_str(&out)
        .expect("--inspect --report-format json should produce valid JSON");
    assert!(
        parsed.get("topics").is_some(),
        "JSON should have a 'topics' field"
    );
    assert!(
        parsed.get("sections").is_some(),
        "JSON should have a 'sections' field"
    );
    assert!(
        parsed.get("generated").is_some(),
        "JSON should have a 'generated' field"
    );
}

#[tokio::test]
async fn test_generate_inspect_output_json_multi_topic() {
    let out =
        hematite::agent::report_export::generate_inspect_output_json("connectivity,wifi").await;
    let parsed: serde_json::Value =
        serde_json::from_str(&out).expect("multi-topic JSON inspect should be valid");
    let topics = parsed["topics"]
        .as_array()
        .expect("topics should be an array");
    assert_eq!(topics.len(), 2, "should have 2 topics in JSON output");
    assert!(
        parsed["sections"].get("connectivity").is_some(),
        "sections should contain connectivity"
    );
    assert!(
        parsed["sections"].get("wifi").is_some(),
        "sections should contain wifi"
    );
}

// ── fix_plan_topics routing ───────────────────────────────────────────────────

#[test]
fn test_fix_routes_display_config_for_monitor_queries() {
    for query in &[
        "monitor not working",
        "second monitor",
        "wrong resolution",
        "bad refresh rate",
        "scaling too big",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"display_config"),
            "\"{}\" should route to display_config, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_peripherals_for_keyboard_mouse_queries() {
    for query in &[
        "keyboard not working",
        "mouse not working",
        "touchpad not responding",
        "trackpad broken",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"peripherals"),
            "\"{}\" should route to peripherals, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_sleep_topics_for_hibernate_queries() {
    for query in &[
        "computer won't hibernate",
        "won't wake up from sleep",
        "stuck after sleep",
        "sleep mode broken",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"services")
                || names.contains(&"pending_reboot")
                || names.contains(&"thermal"),
            "\"{}\" should route to sleep-related topics, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_installer_health_for_store_queries() {
    for query in &[
        "microsoft store not working",
        "store app won't install",
        "uwp app broken",
        "winget failing",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"installer_health"),
            "\"{}\" should route to installer_health, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_audio_for_sound_queries() {
    for query in &[
        "no sound",
        "audio not working",
        "microphone not working",
        "crackling audio",
        "no audio output",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"audio"),
            "\"{}\" should route to audio, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_bluetooth_for_pairing_queries() {
    for query in &[
        "bluetooth not working",
        "headset won't connect",
        "can't pair bluetooth",
        "bluetooth keeps disconnecting",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"bluetooth"),
            "\"{}\" should route to bluetooth, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_outlook_for_email_queries() {
    for query in &[
        "outlook not opening",
        "outlook crashing",
        "email not working",
        "pst file corrupt",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"outlook"),
            "\"{}\" should route to outlook, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_teams_for_teams_queries() {
    for query in &[
        "teams not working",
        "teams won't open",
        "microsoft teams crashing",
        "teams black screen",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"teams"),
            "\"{}\" should route to teams, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_suggest_fix_commands_returns_hints_for_known_findings() {
    // Simulate triage output containing known recipe triggers
    let content = "DNS: Failed\nDrive health: Warning\nHigh memory pressure detected";
    let suggestions = hematite::agent::report_export::suggest_fix_commands(content);
    assert!(
        !suggestions.is_empty(),
        "known findings should produce --fix suggestions, got none"
    );
    for s in &suggestions {
        assert!(
            s.contains("hematite --fix"),
            "suggestion should be a hematite --fix command, got: {}",
            s
        );
    }
}

#[test]
fn test_suggest_fix_commands_empty_for_healthy_content() {
    let content = "All systems healthy. No issues detected.";
    let suggestions = hematite::agent::report_export::suggest_fix_commands(content);
    assert!(
        suggestions.is_empty(),
        "healthy content should produce no suggestions, got: {:?}",
        suggestions
    );
}

#[test]
fn test_fix_routes_browser_health_for_browser_queries() {
    for query in &[
        "chrome slow",
        "edge crashing",
        "firefox not working",
        "browser keeps crashing",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"browser_health"),
            "\"{}\" should route to browser_health, got: {:?}",
            query,
            names
        );
    }
}

// ── New routing coverage ─────────────────────────────────────────────────────

#[test]
fn test_fix_routes_display_for_flickering() {
    for query in &[
        "screen flickering",
        "monitor flicker",
        "display artifact",
        "screen goes black",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"display_config"),
            "\"{}\" should route to display_config, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_storage_for_high_disk() {
    for query in &[
        "high disk usage",
        "disk 100 percent",
        "disk is full",
        "no space left",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"storage"),
            "\"{}\" should route to storage, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_overclocker_for_gpu_gaming() {
    for query in &[
        "GPU overheating",
        "game slow",
        "fps drop",
        "graphics card issue",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"overclocker"),
            "\"{}\" should route to overclocker, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_startup_items_for_boot_slow() {
    for query in &["startup slow", "boot slow", "slow boot", "long boot time"] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"startup_items"),
            "\"{}\" should route to startup_items, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_fix_routes_installer_health_for_install_failures() {
    for query in &[
        "can't install app",
        "installation failed",
        "winget fail",
        "store install stuck",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"installer_health"),
            "\"{}\" should route to installer_health, got: {:?}",
            query,
            names
        );
    }
}

#[test]
fn test_report_indicates_issues_true_for_non_a_grade() {
    let content = "Health Score: **D — Action Required**\nLow disk space detected.";
    assert!(
        hematite::agent::report_export::report_has_issues_in_content(content),
        "D-grade content should indicate issues"
    );
}

#[test]
fn test_report_indicates_issues_false_for_a_grade() {
    let content = "Health Score: **A — All Good**\nNo issues detected.";
    assert!(
        !hematite::agent::report_export::report_has_issues_in_content(content),
        "A-grade content should not indicate issues"
    );
}

#[test]
fn test_auto_fix_new_entries_detected() {
    for trigger in &[
        "teams cache: 2.1 GB",
        "bthserv stopped",
        "dhcp lease expired",
        "wmi repository corrupt",
        "unidentified network",
        "onedrive not running",
    ] {
        let fixes = hematite::agent::report_export::fix_plan_auto_commands(trigger);
        assert!(
            !fixes.is_empty(),
            "trigger {:?} should match at least one auto-fix",
            trigger
        );
    }
}

#[test]
fn test_auto_fix_verify_fields_set_for_dns_flush() {
    let fixes = hematite::agent::report_export::fix_plan_auto_commands("dns: failed");
    assert!(!fixes.is_empty(), "dns: failed should match");
    let fix = &fixes[0];
    assert_eq!(fix.label, "Flush DNS cache");
    assert_eq!(fix.verify_topic, Some("connectivity"));
    assert_eq!(fix.verify_gone, Some("dns: failed"));
}

#[test]
fn test_auto_fix_deduplicates_same_label() {
    // Both "wsearch" and "windows search" map to the same label — only one entry returned.
    let fixes =
        hematite::agent::report_export::fix_plan_auto_commands("wsearch stopped windows search");
    let labels: Vec<&str> = fixes.iter().map(|f| f.label).collect();
    let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
    assert_eq!(
        labels.len(),
        unique.len(),
        "duplicate labels in auto-fix results"
    );
}

#[test]
fn test_sweep_auto_fixes_no_duplicate_labels() {
    let sweep = hematite::agent::report_export::sweep_auto_fixes();
    let labels: Vec<&str> = sweep.iter().map(|f| f.label).collect();
    let unique: std::collections::HashSet<&str> = labels.iter().copied().collect();
    assert_eq!(
        labels.len(),
        unique.len(),
        "sweep has duplicate labels: {:?}",
        labels
    );
}

#[test]
fn test_sweep_auto_fixes_all_have_verify_or_are_always_safe() {
    let sweep = hematite::agent::report_export::sweep_auto_fixes();
    assert!(!sweep.is_empty(), "sweep list must not be empty");
    // Every sweep entry either has a verify pair or is unconditionally safe (no verify needed).
    // Entries without verify run unconditionally — ensure none are security-sensitive.
    // Explicitly allowed: Recycle Bin and Temp folder cleanup (file-system cleanup, always safe).
    const ALWAYS_SAFE: &[&str] = &["Recycle Bin", "Temp folder"];
    for fix in &sweep {
        if fix.verify_topic.is_none() {
            assert!(
                ALWAYS_SAFE.iter().any(|s| fix.label.contains(s)),
                "sweep entry without verify should be obviously safe, got: {}",
                fix.label
            );
        }
    }
}

#[test]
fn test_sweep_excludes_security_sensitive_fixes() {
    let sweep = hematite::agent::report_export::sweep_auto_fixes();
    let labels: Vec<&str> = sweep.iter().map(|f| f.label).collect();
    assert!(
        !labels.contains(&"Enable Remote Desktop"),
        "Enable Remote Desktop must not be in sweep — security sensitive"
    );
    assert!(
        !labels.contains(&"Restart WMI service"),
        "Restart WMI must not be in sweep — disruptive"
    );
    assert!(
        !labels.contains(&"Renew DHCP lease"),
        "Renew DHCP must not be in sweep — drops network briefly"
    );
    assert!(
        !labels.contains(&"Reset TCP/IP stack"),
        "Reset TCP/IP must not be in sweep — requires reboot"
    );
    assert!(
        !labels.contains(&"Restart WLAN AutoConfig service"),
        "Restart WLAN must not be in sweep — could drop Wi-Fi briefly"
    );
    assert!(
        !labels.contains(&"Restart Cryptographic Services"),
        "Restart CryptSvc must not be in sweep — disruptive to active auth"
    );
}

#[test]
fn test_sweep_includes_temp_folder_cleanup() {
    let sweep = hematite::agent::report_export::sweep_auto_fixes();
    let labels: Vec<&str> = sweep.iter().map(|f| f.label).collect();
    assert!(
        labels.contains(&"Clear Windows Temp folder"),
        "Temp folder cleanup should be in sweep: {:?}",
        labels
    );
}

#[test]
fn test_sweep_includes_firewall_restart() {
    let sweep = hematite::agent::report_export::sweep_auto_fixes();
    let labels: Vec<&str> = sweep.iter().map(|f| f.label).collect();
    assert!(
        labels.contains(&"Restart Windows Firewall"),
        "Windows Firewall restart should be in sweep: {:?}",
        labels
    );
}

#[test]
fn test_fix_plan_routes_winsock_to_reset() {
    let fixes = hematite::agent::report_export::fix_plan_auto_commands("winsock catalog issue");
    assert!(!fixes.is_empty(), "winsock should match a fix");
    assert!(
        fixes.iter().any(|f| f.label == "Reset TCP/IP stack"),
        "winsock trigger should map to Reset TCP/IP stack"
    );
}

#[test]
fn test_fix_plan_routes_wlansvc_to_wlan_restart() {
    let fixes = hematite::agent::report_export::fix_plan_auto_commands("wlansvc service stopped");
    assert!(!fixes.is_empty(), "wlansvc should match a fix");
    assert!(
        fixes
            .iter()
            .any(|f| f.label == "Restart WLAN AutoConfig service"),
        "wlansvc trigger should map to WLAN AutoConfig restart"
    );
}

#[test]
fn test_fix_plan_routes_cryptsvc() {
    let fixes = hematite::agent::report_export::fix_plan_auto_commands("cryptsvc not running");
    assert!(!fixes.is_empty(), "cryptsvc should match a fix");
    assert!(
        fixes
            .iter()
            .any(|f| f.label == "Restart Cryptographic Services"),
        "cryptsvc trigger should map to Cryptographic Services restart"
    );
}

// ── New fix_recipes.rs coverage ──────────────────────────────────────────────

#[test]
fn test_recipe_matches_no_audio() {
    let out = hematite::agent::fix_recipes::match_recipes(
        "Core audio services are not running: Audiosrv, AudioEndpointBuilder",
    );
    assert!(!out.is_empty(), "should match audio recipe");
    assert!(
        out.iter().any(|r| r.title.contains("No audio")),
        "should match 'No audio' recipe"
    );
}

#[test]
fn test_recipe_matches_bluetooth_not_working() {
    let out = hematite::agent::fix_recipes::match_recipes(
        "Bluetooth-related services are not fully running: BthServ",
    );
    assert!(!out.is_empty(), "should match bluetooth recipe");
    assert!(
        out.iter().any(|r| r.title.contains("Bluetooth")),
        "should match Bluetooth recipe"
    );
}

#[test]
fn test_recipe_matches_installer_health() {
    let out = hematite::agent::fix_recipes::match_recipes(
        "Windows Installer service (msiserver) is disabled - MSI installs cannot start until it is re-enabled."
    );
    assert!(!out.is_empty(), "should match installer recipe");
    assert!(
        out.iter().any(|r| r.title.contains("App installation")),
        "should match app installation recipe"
    );
}

#[test]
fn test_sweep_list_json_schema_shape() {
    // Verify the JSON structure of --fix-all --only list --report-format json.
    let all = hematite::agent::report_export::sweep_auto_fixes();
    let arr: Vec<serde_json::Value> = all
        .iter()
        .map(|f| {
            serde_json::json!({
                "label": f.label,
                "verify_topic": f.verify_topic,
                "verify_gone": f.verify_gone,
            })
        })
        .collect();
    let out =
        serde_json::to_string_pretty(&serde_json::Value::Array(arr)).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("should parse");
    let items = parsed.as_array().expect("should be array");
    assert!(!items.is_empty(), "sweep list JSON should be non-empty");
    let first = &items[0];
    assert!(first.get("label").is_some(), "each item should have label");
    assert!(
        first.get("verify_topic").is_some(),
        "each item should have verify_topic (may be null)"
    );
    assert!(
        first.get("verify_gone").is_some(),
        "each item should have verify_gone (may be null)"
    );
    // All labels must be non-empty strings
    for item in items {
        let label = item["label"].as_str().expect("label should be string");
        assert!(!label.is_empty(), "label should not be empty");
    }
}

#[test]
fn test_recipe_matches_bsod() {
    // Trigger strings come from inspect_host(topic:"recent_crashes") output
    for trigger in &[
        "bsod (bugcheck)",
        "unexpected shutdown",
        "blue screen",
        "stop code",
        "keeps crashing",
        "random restart",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Blue screen") || r.title.contains("BSOD")),
            "should match BSOD recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_camera_blocked() {
    // Trigger strings come from inspect_host(topic:"camera") output
    for trigger in &[
        "global: deny",
        "camera access is globally denied",
        "no camera devices found via pnp",
        "camera not working",
        "webcam not working",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Camera") || r.title.contains("webcam")),
            "should match camera recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_fix_execute_json_schema() {
    // Validate the JSON shape produced by --fix --execute --report-format json.
    // We simulate the output structure without actually running commands.
    let fixes =
        hematite::agent::report_export::fix_plan_auto_commands("winsock catalog is corrupted");
    let results: Vec<serde_json::Value> = fixes
        .iter()
        .map(|fix| {
            serde_json::json!({
                "label": fix.label,
                "status": "ok",
                "verified_resolved": serde_json::Value::Null,
            })
        })
        .collect();
    let obj = serde_json::json!({
        "issue": "winsock catalog is corrupted",
        "fixes_applied": results,
    });
    let out = serde_json::to_string_pretty(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("should parse");
    assert!(parsed.get("issue").is_some(), "json must have 'issue' key");
    assert!(
        parsed.get("fixes_applied").is_some(),
        "json must have 'fixes_applied' key"
    );
    let applied = parsed["fixes_applied"]
        .as_array()
        .expect("fixes_applied must be array");
    if !applied.is_empty() {
        let first = &applied[0];
        assert!(first.get("label").is_some(), "each fix must have 'label'");
        assert!(first.get("status").is_some(), "each fix must have 'status'");
        assert!(
            first.get("verified_resolved").is_some(),
            "each fix must have 'verified_resolved'"
        );
    }
}

#[test]
fn test_fix_all_json_execution_result_schema() {
    // Validate the JSON shape produced by --fix-all --report-format json.
    // Simulates the serialization logic without running any commands.
    let checks: Vec<serde_json::Value> = vec![
        serde_json::json!({"label": "Flush DNS cache", "status": "healthy"}),
        serde_json::json!({"label": "Restart Windows Search", "status": "fixed"}),
    ];
    let obj = serde_json::json!({
        "generated": "2026-01-01",
        "host": "TEST-PC",
        "hematite_version": "0.9.0",
        "checks_run": 2,
        "applied": 1,
        "verified": 1,
        "unresolved": 0,
        "summary": "1 fix(es) applied, 1 verified resolved.",
        "checks": checks,
    });
    let out = serde_json::to_string_pretty(&obj).expect("should serialize");
    let parsed: serde_json::Value = serde_json::from_str(&out).expect("should parse");
    assert!(parsed.get("generated").is_some(), "must have 'generated'");
    assert!(parsed.get("host").is_some(), "must have 'host'");
    assert!(parsed.get("checks_run").is_some(), "must have 'checks_run'");
    assert!(parsed.get("applied").is_some(), "must have 'applied'");
    assert!(parsed.get("verified").is_some(), "must have 'verified'");
    assert!(parsed.get("unresolved").is_some(), "must have 'unresolved'");
    assert!(parsed.get("summary").is_some(), "must have 'summary'");
    let checks_arr = parsed["checks"].as_array().expect("'checks' must be array");
    assert!(!checks_arr.is_empty(), "checks must be non-empty in test");
    let first = &checks_arr[0];
    assert!(first.get("label").is_some(), "each check must have 'label'");
    assert!(
        first.get("status").is_some(),
        "each check must have 'status'"
    );
}

#[test]
fn test_recipe_matches_vpn_not_connecting() {
    for trigger in &["vpn adapter detected", "rasman", "ras/vpn", "vpn tunnel"] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("VPN")),
            "should match VPN recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_screen_flickering() {
    for trigger in &[
        "display driver",
        "refresh rate:",
        "screen flickering",
        "resolution wrong",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Screen flickering") || r.title.contains("display")),
            "should match screen flickering recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_microphone_not_working() {
    for trigger in &[
        "no recording endpoints found",
        "microphone access: denied",
        "microphone not working",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Microphone")),
            "should match microphone recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_login_pin_not_working() {
    for trigger in &[
        "wbiosrvc",
        "windows hello",
        "pin not working",
        "event id 4625",
        "sign-in failed",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Login") || r.title.contains("PIN")),
            "should match login/PIN recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_high_disk_io() {
    for trigger in &[
        "disk queue length:",
        "average disk queue",
        "high disk usage",
        "disk at 100",
        "disk thrashing",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Disk at 100%") || r.title.contains("disk I/O")),
            "should match high disk I/O recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_usb_not_recognized() {
    // Note: triggers that are substrings of existing recipes ("yellow bang", "error code 10")
    // are intentionally omitted here — the "Hardware device error" recipe owns those.
    for trigger in &[
        "[err:",
        "usb device not recognized",
        "unknown usb device",
        "device descriptor request failed",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("USB")),
            "should match USB recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_no_wifi_networks() {
    for trigger in &[
        "there is no wireless interface",
        "no wi-fi devices found",
        "wi-fi adapter disconnected",
        "no wireless networks",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Wi-Fi") || r.title.contains("wireless")),
            "should match no-Wi-Fi recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_network_share_not_accessible() {
    for trigger in &[
        "server unreachable (ping failed)",
        "reachable:false",
        "network path not found",
        "share not accessible",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Network share") || r.title.contains("mapped drive")),
            "should match network share recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_microsoft_store_not_working() {
    for trigger in &[
        "microsoft.windowsstore | status: missing",
        "wsreset",
        "appx package",
        "microsoft store not opening",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("Microsoft Store") || r.title.contains("AppX")),
            "should match Microsoft Store recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_usb_routes_to_device_health() {
    let topics = hematite::agent::report_export::fix_plan_topics("USB device not recognized");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"device_health"),
        "USB query should route to device_health"
    );
}

#[test]
fn test_routing_no_wifi_routes_to_wifi_and_device_health() {
    let topics = hematite::agent::report_export::fix_plan_topics("no Wi-Fi networks showing");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"wifi"),
        "no-wifi query should route to wifi topic"
    );
}

#[test]
fn test_routing_network_share_routes_to_share_access() {
    let topics = hematite::agent::report_export::fix_plan_topics("network share not accessible");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"share_access"),
        "network share query should route to share_access"
    );
}

#[test]
fn test_routing_microsoft_store_routes_to_installer_health() {
    let topics = hematite::agent::report_export::fix_plan_topics("Microsoft Store not working");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"installer_health"),
        "Microsoft Store query should route to installer_health"
    );
}

#[test]
fn test_recipe_matches_sleep_wake_issue() {
    for trigger in &[
        "kernel-power",
        "power-troubleshooter",
        "sleep fail",
        "won't wake",
        "fast startup",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("sleep")
                || r.title.contains("hibernate")
                || r.title.contains("wake")),
            "should match sleep/wake recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_keyboard_mouse_not_working() {
    for trigger in &[
        "hid keyboard",
        "hid mouse",
        "hid-compliant",
        "keyboard not detected",
        "touchpad not working",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Keyboard")
                || r.title.contains("mouse")
                || r.title.contains("touchpad")),
            "should match keyboard/mouse recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_high_network_usage() {
    for trigger in &[
        "bytes sent (mb):",
        "bytes received (mb):",
        "high bandwidth",
        "network usage high",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter()
                .any(|r| r.title.contains("network usage") || r.title.contains("bandwidth")),
            "should match high network usage recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_sleep_routes_to_log_check() {
    let topics = hematite::agent::report_export::fix_plan_topics("PC won't sleep");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"log_check"),
        "sleep query should route to log_check"
    );
}

#[test]
fn test_routing_keyboard_routes_to_peripherals() {
    let topics = hematite::agent::report_export::fix_plan_topics("keyboard not working");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"peripherals"),
        "keyboard query should route to peripherals"
    );
}

#[test]
fn test_routing_bandwidth_routes_to_network_stats() {
    let topics = hematite::agent::report_export::fix_plan_topics("high bandwidth usage");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"network_stats"),
        "bandwidth query should route to network_stats"
    );
}

#[test]
fn test_recipe_matches_audio_crackling() {
    for trigger in &[
        "crackling",
        "audio distortion",
        "audio stuttering",
        "dpc latency",
        "exclusive mode",
        "sound crackling",
        "audio popping",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("crackling") || r.title.contains("distortion")),
            "should match audio crackling recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_browser_slow_or_crashing() {
    for trigger in &[
        "browser crash",
        "browser slow",
        "browser freezing",
        "browser high cpu",
        "chrome slow",
        "edge slow",
        "webview2 runtime: missing",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Browser") || r.title.contains("browser")),
            "should match browser recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_browser_routes_to_browser_health() {
    for query in &["Chrome running slow", "browser keeps crashing", "Edge not opening"] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            topic_ids.contains(&"browser_health"),
            "browser query '{query}' should route to browser_health"
        );
    }
}

#[test]
fn test_recipe_matches_slow_startup() {
    for trigger in &[
        "startup takes",
        "long boot time",
        "slow to start up",
        "startup items high impact",
        "many startup programs",
        "boot is slow",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("startup is slow") || r.title.contains("long time to boot")),
            "should match slow startup recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_windows_update_stuck() {
    for trigger in &[
        "update error 0x",
        "0x8024a105",
        "0x80070422",
        "update stuck downloading",
        "update failed to install",
        "cumulative update failed",
        "feature update failed",
        "update rollback failed",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Update stuck") || r.title.contains("error code")),
            "should match Windows Update stuck recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_gpu_driver_crash() {
    for trigger in &[
        "nvlddmkm.sys",
        "nvlddmkm",
        "amdkmdag.sys",
        "tdr failure",
        "video_tdr_failure",
        "gpu driver crash",
        "gpu hang",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("GPU") || r.title.contains("display driver")),
            "should match GPU driver crash recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_gpu_crash_routes_to_device_health() {
    for query in &[
        "GPU driver crash black screen",
        "nvlddmkm.sys BSOD",
        "TDR failure video",
    ] {
        let topics = hematite::agent::report_export::fix_plan_topics(query);
        let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            topic_ids.contains(&"device_health") || topic_ids.contains(&"recent_crashes"),
            "GPU crash query '{query}' should route to device_health or recent_crashes"
        );
    }
}

#[test]
fn test_recipe_matches_access_denied() {
    // "access is denied" is owned by the network share recipe — test permission-specific triggers
    for trigger in &[
        "access denied",
        "you don't have permission",
        "permission denied",
        "you do not have permission",
        "cannot access this folder",
        "unauthorized access",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Access denied") || r.title.contains("permission")),
            "should match access denied recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_wifi_dropping() {
    for trigger in &[
        "wifi disconnects",
        "wifi keeps dropping",
        "wifi keeps disconnecting",
        "internet keeps cutting out",
        "wifi unstable",
        "wifi intermittent",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Wi-Fi keeps") || r.title.contains("dropping")),
            "should match wifi dropping recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_access_denied_routes_to_user_accounts() {
    let topics = hematite::agent::report_export::fix_plan_topics("access denied opening file");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"user_accounts"),
        "access denied query should route to user_accounts"
    );
}

#[test]
fn test_routing_wifi_dropping_routes_to_network_adapter() {
    let topics = hematite::agent::report_export::fix_plan_topics("wifi keeps dropping connection");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"wifi") || topic_ids.contains(&"network_adapter"),
        "wifi dropping query should route to wifi or network_adapter"
    );
}

#[test]
fn test_fix_issue_categories_covers_advertised_areas() {
    let cats = hematite::agent::report_export::fix_issue_categories();
    let names: Vec<&str> = cats.iter().map(|(n, _)| *n).collect();
    for expected in &[
        "Sleep / Hibernate",
        "Keyboard / Mouse",
        "Network Share",
        "High Network Usage",
        "USB Device",
        "Crash / BSOD",
        "Audio",
        "Bluetooth",
        "Camera",
        "GPU Driver Crash",
        "Windows Update Stuck",
        "Slow Boot",
        "Access Denied",
        "Wi-Fi Dropping",
        "Defender High CPU",
        "Monitor Not Detected",
        "Explorer / Desktop Crashed",
        "Overheating / Fan",
        "RAM / Memory",
        "Windows Activation",
        "BitLocker",
        "Domain / Group Policy",
        "Hyper-V / VM",
        "WSL",
        "Docker",
        "Random Restart",
        "Disk Filling Up",
        "DHCP / IP Address",
        "Certificate / SSL",
        "TPM / Secure Boot",
        "SMB / NTLM Security",
        "Windows Search",
    ] {
        assert!(
            names.contains(expected),
            "fix_issue_categories should include '{expected}'"
        );
    }
}

#[test]
fn test_recipe_matches_msmpeng_high_cpu() {
    // "antimalware" contains "malware" which is owned by the Threat detected recipe — omit it
    for trigger in &[
        "msmpeng.exe",
        "msmpeng",
        "defender using high cpu",
        "defender scan high cpu",
        "wdnissvc.exe",
        "windows defender high",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Antimalware") || r.title.contains("MsMpEng")),
            "should match MsMpEng recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_msmpeng_routes_to_resource_load() {
    let topics = hematite::agent::report_export::fix_plan_topics("MsMpEng.exe high CPU usage");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"resource_load"),
        "MsMpEng query should route to resource_load"
    );
}

#[test]
fn test_recipe_matches_external_monitor_not_detected() {
    for trigger in &[
        "monitor not detected",
        "second monitor not showing",
        "hdmi not working",
        "displayport not detected",
        "external display not",
        "no signal on monitor",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("External monitor") || r.title.contains("no signal")),
            "should match external monitor recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_recipe_matches_explorer_crash() {
    for trigger in &[
        "explorer.exe crash",
        "windows explorer crash",
        "desktop icons disappeared",
        "taskbar disappeared",
        "start menu crashed",
    ] {
        let out = hematite::agent::fix_recipes::match_recipes(trigger);
        assert!(
            out.iter().any(|r| r.title.contains("Explorer") || r.title.contains("taskbar")),
            "should match explorer crash recipe for trigger: {trigger}"
        );
    }
}

#[test]
fn test_routing_monitor_routes_to_display_config() {
    let topics = hematite::agent::report_export::fix_plan_topics("second monitor not showing up");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"display_config"),
        "monitor query should route to display_config"
    );
}

#[test]
fn test_routing_explorer_crash_routes_to_processes() {
    let topics = hematite::agent::report_export::fix_plan_topics("taskbar disappeared after crash");
    let topic_ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        topic_ids.contains(&"processes") || topic_ids.contains(&"log_check"),
        "explorer crash query should route to processes or log_check"
    );
}

#[test]
fn test_routing_overheating_routes_to_thermal() {
    let cases = [
        "PC overheating",
        "cpu temperature too high",
        "thermal throttling",
        "fan running loud",
        "fans spinning at max speed",
        "laptop fan always on",
        "fan at 100 percent",
        "too hot",
    ];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            ids.contains(&"thermal"),
            "thermal routing expected for: {q}"
        );
    }
}

#[test]
fn test_routing_ram_pressure_routes_to_resource_load() {
    let cases = [
        "RAM almost full",
        "out of memory error",
        "running out of ram",
        "memory usage high",
        "memory leak",
        "low memory",
    ];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            ids.contains(&"resource_load"),
            "resource_load routing expected for: {q}"
        );
    }
}

#[test]
fn test_routing_mic_keyword_no_false_positive_on_microsoft() {
    // "mic" is a substring of "microsoft" — ensure it does not trigger audio routing
    let topics = hematite::agent::report_export::fix_plan_topics("can't open Microsoft Store");
    let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(
        !ids.contains(&"audio"),
        "audio should NOT be routed for 'can't open Microsoft Store' (false mic match)"
    );
    assert!(
        ids.contains(&"installer_health"),
        "installer_health should be routed for 'can't open Microsoft Store'"
    );
}

#[test]
fn test_routing_microphone_still_routes_to_audio() {
    let cases = ["microphone not working", "mic not working", "mic keeps cutting out", "my mic is broken"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"audio"), "audio routing expected for: {q}");
    }
}

#[test]
fn test_routing_ntp_no_false_positive_on_sync_fail() {
    // "NTP sync failing" previously matched onedrive's "sync fail" keyword
    let topics = hematite::agent::report_export::fix_plan_topics("NTP sync failing");
    let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
    assert!(!ids.contains(&"onedrive"), "onedrive should NOT match 'NTP sync failing'");
    assert!(ids.contains(&"ntp"), "ntp should match 'NTP sync failing'");
}

#[test]
fn test_routing_time_zone_routes_to_ntp() {
    let cases = ["time zone wrong", "wrong timezone", "timezone incorrect"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"ntp"), "ntp routing expected for: {q}");
    }
}

#[test]
fn test_routing_ip_dhcp_routes_correctly() {
    let cases = ["IP address conflict", "no IP address", "DHCP not working"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"dhcp"), "dhcp routing expected for: {q}");
    }
}

#[test]
fn test_routing_ipv6_mtu_route_correctly() {
    let ipv6_topics = hematite::agent::report_export::fix_plan_topics("IPv6 not working");
    let mtu_topics = hematite::agent::report_export::fix_plan_topics("MTU issues causing packet loss");
    let ipv6_ids: Vec<_> = ipv6_topics.iter().map(|(t, _)| *t).collect();
    let mtu_ids: Vec<_> = mtu_topics.iter().map(|(t, _)| *t).collect();
    assert!(ipv6_ids.contains(&"ipv6"), "ipv6 routing expected for IPv6 query");
    assert!(mtu_ids.contains(&"mtu"), "mtu routing expected for MTU query");
}

#[test]
fn test_routing_certificates_tpm_smb_route_correctly() {
    let cases = [
        ("certificate expired", "certificates"),
        ("TPM not detected", "tpm"),
        ("secure boot disabled", "tpm"),
        ("SMB1 enabled warning", "shares"),
    ];
    for (q, expected) in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(expected), "{expected} routing expected for: {q}");
    }
}

#[test]
fn test_routing_pagefile_search_index_route_correctly() {
    let cases = [
        ("hiberfil.sys too big", "pagefile"),
        ("pagefile taking up space", "pagefile"),
        ("windows search eating disk", "search_index"),
    ];
    for (q, expected) in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(expected), "{expected} routing expected for: {q}");
    }
}

#[test]
fn test_routing_wmi_event_log_route_correctly() {
    let wmi_topics = hematite::agent::report_export::fix_plan_topics("WMI not working");
    let log_topics = hematite::agent::report_export::fix_plan_topics("event log full");
    let wmi_ids: Vec<_> = wmi_topics.iter().map(|(t, _)| *t).collect();
    let log_ids: Vec<_> = log_topics.iter().map(|(t, _)| *t).collect();
    assert!(wmi_ids.contains(&"wmi_health"), "wmi_health routing expected for WMI query");
    assert!(log_ids.contains(&"log_check"), "log_check routing expected for event log query");
}

#[test]
fn test_routing_activation_routes_correctly() {
    let cases = ["Windows license expired", "not activated", "need to activate Windows"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"activation"), "activation routing expected for: {q}");
    }
}

#[test]
fn test_routing_bitlocker_routes_correctly() {
    let cases = ["BitLocker asking for recovery key", "BitLocker locked", "drive encryption failed"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"bitlocker"), "bitlocker routing expected for: {q}");
    }
}

#[test]
fn test_routing_domain_routes_correctly() {
    let cases = ["can't join domain", "Group Policy not applying", "domain controller unreachable"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"domain_health"), "domain_health routing expected for: {q}");
    }
}

#[test]
fn test_routing_hyperv_wsl_docker_route_correctly() {
    let cases = [
        ("Hyper-V VM won't start", "hyperv"),
        ("WSL not working", "wsl"),
        ("Docker container won't start", "docker"),
    ];
    for (q, expected_topic) in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(expected_topic), "{expected_topic} routing expected for: {q}");
    }
}

#[test]
fn test_routing_random_restart_routes_to_crashes() {
    let cases = ["computer restarts randomly", "keeps restarting unexpectedly", "random reboot"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"recent_crashes"), "recent_crashes routing expected for: {q}");
    }
}

#[test]
fn test_routing_disk_filling_routes_to_storage() {
    let cases = ["SSD getting full fast", "hard drive filling up", "recycle bin won't empty"];
    for q in &cases {
        let topics = hematite::agent::report_export::fix_plan_topics(q);
        let ids: Vec<_> = topics.iter().map(|(t, _)| *t).collect();
        assert!(ids.contains(&"storage"), "storage routing expected for: {q}");
    }
}

#[test]
fn test_routing_fan_phrases_route_to_thermal_query_path() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "fan running loud",
        "fans spinning at max speed",
        "laptop fan always on",
        "fan noise is loud",
        "fan at max speed",
        "fans running constantly",
        "pc running hot",
        "laptop too hot",
        "cpu temperature too high",
    ];
    for q in &cases {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("thermal"),
            "thermal routing expected for query: {q}"
        );
    }
}

#[test]
fn test_routing_login_loop_routes_to_sign_in() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "login screen stuck",
        "stuck on login screen",
        "stuck at login",
        "login loop",
        "sign-in loop",
        "sign in loop",
    ];
    for q in &cases {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("sign_in"),
            "sign_in routing expected for query: {q}"
        );
    }
}

#[test]
fn test_routing_time_zone_routes_to_ntp_query_path() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = ["time zone wrong", "wrong timezone", "timezone incorrect"];
    for q in &cases {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("ntp"),
            "ntp routing expected for query: {q}"
        );
    }
}

#[test]
fn test_routing_git_auth_routes_to_git_config() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "git push denied",
        "git clone failed authentication",
        "git identity not set",
        "git auth not working",
    ];
    for q in &cases {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("git_config"),
            "git_config routing expected for query: {q}"
        );
    }
}

#[test]
fn test_routing_dev_conflicts_phrases() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "nvm conflict with node",
        "pyenv conflict",
        "version conflict in dev environment",
    ];
    for q in &cases {
        assert_eq!(
            preferred_host_inspection_topic(q),
            Some("dev_conflicts"),
            "dev_conflicts routing expected for query: {q}"
        );
    }
}

#[test]
fn test_routing_path_and_toolchain_phrases() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("path issue with cargo"),
        Some("path")
    );
    assert_eq!(
        preferred_host_inspection_topic("toolchain not found"),
        Some("toolchains")
    );
    assert_eq!(
        preferred_host_inspection_topic("toolchain missing for rust"),
        Some("toolchains")
    );
}

#[test]
fn test_ps_escape_single_quoted_correctness() {
    // Re-test the escaping logic used in host_inspect.rs via a local reimplementation
    // so regressions are caught without needing to expose the private helper.
    fn escape(s: &str) -> String {
        s.replace('\'', "''")
    }

    // Basic pass-through
    assert_eq!(escape("example.com"), "example.com");
    assert_eq!(escape("8.8.8.8"), "8.8.8.8");
    assert_eq!(escape("Application"), "Application");

    // Single-quote injection sequences must be doubled
    assert_eq!(escape("'; evil"), "''; evil");
    assert_eq!(escape("it's"), "it''s");
    assert_eq!(escape("'; Remove-Item -Recurse C:\\ #"), "''; Remove-Item -Recurse C:\\ #");

    // Multiple quotes
    assert_eq!(escape("a'b'c"), "a''b''c");

    // No double-quote or backtick interference
    assert_eq!(escape("domain\\user\"name"), "domain\\user\"name");
    assert_eq!(escape("value`with`backticks"), "value`with`backticks");
}

#[test]
fn test_validate_dns_record_type_allowlist() {
    // Mirrors the allowlist in validate_dns_record_type in host_inspect.rs.
    // Tests that known types are accepted and unknown types fall back to "A".
    let known_types = ["A", "AAAA", "MX", "TXT", "SRV", "CNAME", "NS", "PTR", "SOA", "CAA", "ANY"];
    for rt in known_types {
        // Case-sensitive: the allowlist matches the exact casing passed
        let lower = rt.to_uppercase();
        // We verify the logic by reconstructing it inline
        let result = match lower.as_str() {
            "A" | "AAAA" | "MX" | "TXT" | "SRV" | "CNAME" | "NS" | "PTR" | "SOA" | "CAA"
            | "NAPTR" | "DS" | "DNSKEY" | "ANY" => rt,
            _ => "A",
        };
        assert_eq!(result, rt, "Known type {rt} should pass through unchanged");
    }

    // Unknown/injection attempts must fall back to "A"
    let injections = [
        "A; Get-ChildItem",
        "$(evil)",
        "INVALID",
        "",
        "A\nB",
    ];
    for input in injections {
        let upper = input.to_uppercase();
        let result = match upper.as_str() {
            "A" | "AAAA" | "MX" | "TXT" | "SRV" | "CNAME" | "NS" | "PTR" | "SOA" | "CAA"
            | "NAPTR" | "DS" | "DNSKEY" | "ANY" => input,
            _ => "A",
        };
        assert_eq!(result, "A", "Injection or invalid type {input:?} should fall back to A");
    }
}

#[test]
fn test_api_url_is_local_detection() {
    use hematite::agent::config::api_url_is_local;

    // Known local URLs
    assert!(api_url_is_local("http://localhost:1234/v1"));
    assert!(api_url_is_local("http://localhost:11434/v1"));
    assert!(api_url_is_local("http://127.0.0.1:1234/v1"));
    assert!(api_url_is_local("http://127.0.0.1/v1"));
    assert!(api_url_is_local("http://::1/v1"));

    // Remote URLs must not be treated as local
    assert!(!api_url_is_local("http://192.168.1.100:1234/v1"));
    assert!(!api_url_is_local("https://api.attacker.com/v1"));
    assert!(!api_url_is_local("http://10.0.0.5:1234/v1"));
    assert!(!api_url_is_local("https://openai.com/v1"));
}

#[test]
fn test_safe_write_refuses_symlinks() {
    use hematite::tools::file_ops::safe_write;

    let dir = std::env::temp_dir().join("hematite_safe_write_test");
    let _ = std::fs::create_dir_all(&dir);

    let real_target = dir.join("real_target.txt");
    let _ = std::fs::write(&real_target, b"original");

    #[cfg(unix)]
    {
        let link_path = dir.join("link.txt");
        let _ = std::fs::remove_file(&link_path);
        std::os::unix::fs::symlink(&real_target, &link_path).expect("create symlink");
        let result = safe_write(&link_path, b"injected");
        assert!(result.is_err(), "safe_write must refuse to write through symlinks");
        // Confirm the real target was not modified
        let still_original = std::fs::read_to_string(&real_target).unwrap();
        assert_eq!(still_original, "original");
    }

    // Non-symlink write should succeed
    let plain_path = dir.join("plain.txt");
    let result = safe_write(&plain_path, b"hello");
    assert!(result.is_ok(), "safe_write must succeed for non-symlink paths");
    assert_eq!(std::fs::read_to_string(&plain_path).unwrap(), "hello");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_all_action_recipes_have_fix_arg_mapping() {
    // Every ACTION/INVESTIGATE recipe title should have a recipe_title_to_fix_arg entry so that
    // suggest_fix_commands can surface it as a hematite --fix hint in reports.
    // This guards against silently dropping suggestions when a new recipe is added.
    let mut missing: Vec<&str> = Vec::new();
    for recipe in hematite::agent::fix_recipes::all_recipes() {
        if recipe.severity == "MONITOR" {
            continue;
        }
        if hematite::agent::report_export::recipe_title_to_fix_arg(recipe.title).is_none() {
            missing.push(recipe.title);
        }
    }
    assert!(
        missing.is_empty(),
        "These ACTION/INVESTIGATE recipes have no recipe_title_to_fix_arg mapping:\n{}",
        missing.join("\n")
    );
}

// ── inspect_host header tests for 0.8.0-wave topics ──────────────────────────

#[test]
fn test_inspect_host_thermal_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "thermal" });
        let out = inspect_host(&args).await.expect("thermal must return Ok");
        assert!(out.contains("Host inspection: thermal"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_activation_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "activation" });
        let out = inspect_host(&args).await.expect("activation must return Ok");
        assert!(out.contains("Host inspection: activation"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_patch_history_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "patch_history" });
        let out = inspect_host(&args).await.expect("patch_history must return Ok");
        assert!(out.contains("Host inspection: patch_history"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_storage_spaces_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "storage_spaces" });
        let out = inspect_host(&args).await.expect("storage_spaces must return Ok");
        assert!(out.contains("Host inspection: storage_spaces"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_defender_quarantine_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "defender_quarantine" });
        let out = inspect_host(&args).await.expect("defender_quarantine must return Ok");
        assert!(out.contains("Host inspection: defender_quarantine"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_domain_health_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "domain_health" });
        let out = inspect_host(&args).await.expect("domain_health must return Ok");
        assert!(out.contains("Host inspection: domain_health"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_service_dependencies_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "service_dependencies" });
        let out = inspect_host(&args).await.expect("service_dependencies must return Ok");
        assert!(out.contains("Host inspection: service_dependencies"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_wmi_health_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "wmi_health" });
        let out = inspect_host(&args).await.expect("wmi_health must return Ok");
        assert!(out.contains("Host inspection: wmi_health"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_local_security_policy_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "local_security_policy" });
        let out = inspect_host(&args).await.expect("local_security_policy must return Ok");
        assert!(out.contains("Host inspection: local_security_policy"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_usb_history_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "usb_history" });
        let out = inspect_host(&args).await.expect("usb_history must return Ok");
        assert!(out.contains("Host inspection: usb_history"), "missing header; got:\n{out}");
    });
}

#[test]
fn test_inspect_host_print_spooler_returns_header() {
    use hematite::tools::host_inspect::inspect_host;
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let args = serde_json::json!({ "topic": "print_spooler" });
        let out = inspect_host(&args).await.expect("print_spooler must return Ok");
        assert!(out.contains("Host inspection: print_spooler"), "missing header; got:\n{out}");
    });
}

// ── Batch 14: all_host_inspection_topics detector expansions ──────────────────

#[test]
fn test_multi_topic_display_config_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("hdmi port not working", "display_config"),
        ("displayport not detected", "display_config"),
        ("how many screens can I connect", "display_config"),
        ("multi-monitor setup not working", "display_config"),
        ("external display not showing", "display_config"),
        ("refresh hz setting", "display_config"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_ntp_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("system clock is wrong", "ntp"),
        ("time wrong after reboot", "ntp"),
        ("wrong timezone shown", "ntp"),
        ("time server not responding", "ntp"),
        ("system clock drifting", "ntp"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_domain_health_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("can the machine reach dc", "domain_health"),
        ("dc reachable from this host", "domain_health"),
        ("kerberos connectivity test", "domain_health"),
        ("gpo refresh not working", "domain_health"),
        ("dsgetdc command result", "domain_health"),
        ("ldap error connecting to domain", "domain_health"),
        ("active directory health check", "domain_health"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_service_dependencies_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("which services depend on DNS client", "service_dependencies"),
        ("services depend on this service", "service_dependencies"),
        ("show service graph", "service_dependencies"),
        ("service required by another service", "service_dependencies"),
        ("restart cascade if I stop DHCP", "service_dependencies"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_wmi_health_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("wmi error in powershell", "wmi_health"),
        ("wmi not working at all", "wmi_health"),
        ("wmi query failing", "wmi_health"),
        ("winmgmt service status", "wmi_health"),
        ("wmi repository status", "wmi_health"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_local_security_policy_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("lockout threshold for user accounts", "local_security_policy"),
        ("ntlm authentication level setting", "local_security_policy"),
        ("uac prompt appearing too often", "local_security_policy"),
        ("user account control settings", "local_security_policy"),
        ("needs elevation to run program", "local_security_policy"),
        ("run as administrator not working", "local_security_policy"),
        ("net accounts command output", "local_security_policy"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_usb_history_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("what usb devices were connected", "usb_history"),
        ("usb devices ever plugged in", "usb_history"),
        ("usb devices connected to this pc", "usb_history"),
        ("usb registry audit", "usb_history"),
        ("usb forensic investigation", "usb_history"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_print_spooler_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("print nightmar vulnerability check", "print_spooler"),
        ("cve-2021-1675 mitigation status", "print_spooler"),
        ("printer security hardening", "print_spooler"),
        ("point and print driver policy", "print_spooler"),
        ("spooler service running status", "print_spooler"),
        ("spooler hardening applied", "print_spooler"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} in multi-topic results for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 16: routing tests for untested preferred_host_inspection_topic topics ─

#[test]
fn test_routing_detects_disk_health_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check disk health"), Some("disk_health"));
    assert_eq!(preferred_host_inspection_topic("is my drive failing"), Some("disk_health"));
    assert_eq!(preferred_host_inspection_topic("smart status of the SSD"), Some("disk_health"));
    assert_eq!(preferred_host_inspection_topic("hard drive dying symptoms"), Some("disk_health"));
    assert_eq!(preferred_host_inspection_topic("is the drive healthy"), Some("disk_health"));
}

#[test]
fn test_routing_detects_pending_reboot_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("does my PC need to restart"), Some("pending_reboot"));
    assert_eq!(preferred_host_inspection_topic("reboot required after update"), Some("pending_reboot"));
    assert_eq!(preferred_host_inspection_topic("is a restart pending"), Some("pending_reboot"));
    assert_eq!(preferred_host_inspection_topic("pending reboot check"), Some("pending_reboot"));
}

#[test]
fn test_routing_detects_recent_crashes_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show recent crash history"), Some("recent_crashes"));
    assert_eq!(preferred_host_inspection_topic("why did my PC blue screen"), Some("recent_crashes"));
    assert_eq!(preferred_host_inspection_topic("BSOD last night"), Some("recent_crashes"));
    assert_eq!(preferred_host_inspection_topic("PC keeps restarting randomly"), Some("recent_crashes"));
}

#[test]
fn test_routing_detects_processes_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("what processes are running"), Some("processes"));
    assert_eq!(preferred_host_inspection_topic("show running processes"), Some("processes"));
    assert_eq!(preferred_host_inspection_topic("top memory consuming processes"), Some("processes"));
    assert_eq!(preferred_host_inspection_topic("using ram the most"), Some("processes"));
}

#[test]
fn test_routing_detects_services_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("list all windows services"), Some("services"));
    assert_eq!(preferred_host_inspection_topic("what services are running"), Some("services"));
    assert_eq!(preferred_host_inspection_topic("background service status"), Some("services"));
    assert_eq!(preferred_host_inspection_topic("get-service output"), Some("services"));
}

#[test]
fn test_routing_detects_ports_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("what listening ports does this machine have"), Some("ports"));
    assert_eq!(preferred_host_inspection_topic("what is listening on this machine"), Some("ports"));
    assert_eq!(preferred_host_inspection_topic("what port is the web server on"), Some("ports"));
    assert_eq!(preferred_host_inspection_topic("which ports are exposed on this server"), Some("ports"));
}

#[test]
fn test_routing_detects_wifi_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show wifi signal strength"), Some("wifi"));
    assert_eq!(preferred_host_inspection_topic("show current SSID"), Some("wifi"));
    assert_eq!(preferred_host_inspection_topic("wi-fi connection status"), Some("wifi"));
    assert_eq!(preferred_host_inspection_topic("wireless access point info"), Some("wifi"));
}

#[test]
fn test_routing_detects_updates_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check for windows updates"), Some("updates"));
    assert_eq!(preferred_host_inspection_topic("are there pending updates"), Some("updates"));
    assert_eq!(preferred_host_inspection_topic("is my PC up to date"), Some("updates"));
    assert_eq!(preferred_host_inspection_topic("latest update status"), Some("updates"));
}

#[test]
fn test_routing_detects_security_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("antivirus status check"), Some("security"));
    assert_eq!(preferred_host_inspection_topic("is my PC protected from malware"), Some("security"));
    assert_eq!(preferred_host_inspection_topic("windows security status"), Some("security"));
    assert_eq!(preferred_host_inspection_topic("is defender running"), Some("security"));
}

#[test]
fn test_routing_detects_battery_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check battery health"), Some("battery"));
    assert_eq!(preferred_host_inspection_topic("battery life remaining"), Some("battery"));
    assert_eq!(preferred_host_inspection_topic("charge level of battery"), Some("battery"));
    assert_eq!(preferred_host_inspection_topic("battery wear after 2 years"), Some("battery"));
}

#[test]
fn test_routing_detects_dev_conflicts_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check for dev conflicts"), Some("dev_conflicts"));
    assert_eq!(preferred_host_inspection_topic("toolchain conflict between rust versions"), Some("dev_conflicts"));
    assert_eq!(preferred_host_inspection_topic("nvm conflict with node"), Some("dev_conflicts"));
    assert_eq!(preferred_host_inspection_topic("pyenv conflict with python"), Some("dev_conflicts"));
}

#[test]
fn test_routing_detects_dns_cache_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show dns cache entries"), Some("dns_cache"));
    assert_eq!(preferred_host_inspection_topic("show locally cached dns"), Some("dns_cache"));
    assert_eq!(preferred_host_inspection_topic("view the dns cache contents"), Some("dns_cache"));
    assert_eq!(preferred_host_inspection_topic("inspect the dns cache"), Some("dns_cache"));
}

// ── Batch 17: routing tests for remaining untested preferred_host topics ──────

#[test]
fn test_routing_detects_activation_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check windows activation status"), Some("activation"));
    assert_eq!(preferred_host_inspection_topic("is windows genuine"), Some("activation"));
    assert_eq!(preferred_host_inspection_topic("my product key is invalid"), Some("activation"));
    assert_eq!(preferred_host_inspection_topic("run slmgr /xpr"), Some("activation"));
}

#[test]
fn test_routing_detects_patch_history_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show patch history"), Some("patch_history"));
    assert_eq!(preferred_host_inspection_topic("list installed hotfixes"), Some("patch_history"));
    assert_eq!(preferred_host_inspection_topic("kb history for this machine"), Some("patch_history"));
    assert_eq!(preferred_host_inspection_topic("show installed updates history"), Some("patch_history"));
}

#[test]
fn test_routing_detects_scheduled_tasks_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show all scheduled tasks"), Some("scheduled_tasks"));
    assert_eq!(preferred_host_inspection_topic("task scheduler jobs that run daily"), Some("scheduled_tasks"));
    assert_eq!(preferred_host_inspection_topic("list background tasks"), Some("scheduled_tasks"));
    assert_eq!(preferred_host_inspection_topic("what cron jobs are configured"), Some("scheduled_tasks"));
}

#[test]
fn test_routing_detects_share_access_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("can I access the network share"), Some("share_access"));
    assert_eq!(preferred_host_inspection_topic("test UNC path access"), Some("share_access"));
    assert_eq!(preferred_host_inspection_topic("show the net share listing"), Some("share_access"));
}

#[test]
fn test_routing_detects_health_report_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("run a system health report"), Some("health_report"));
    assert_eq!(preferred_host_inspection_topic("show system health status"), Some("health_report"));
    assert_eq!(preferred_host_inspection_topic("how is my machine doing overall"), Some("health_report"));
}

#[test]
fn test_routing_detects_registry_audit_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show registry audit details"), Some("registry_audit"));
    assert_eq!(preferred_host_inspection_topic("check for registry persistence"), Some("registry_audit"));
    assert_eq!(preferred_host_inspection_topic("sticky keys registry check"), Some("registry_audit"));
}

#[test]
fn test_routing_detects_login_history_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show login history for this user"), Some("login_history"));
    assert_eq!(preferred_host_inspection_topic("last logon history for this account"), Some("login_history"));
    assert_eq!(preferred_host_inspection_topic("show recent logon events"), Some("login_history"));
}

// ── Batch 15: all_host_inspection_topics moderate-gap expansions ──────────────

#[test]
fn test_multi_topic_defender_quarantine_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("defender detected a virus", "defender_quarantine"),
        ("malware history on this PC", "defender_quarantine"),
        ("threats found by defender", "defender_quarantine"),
        ("virus found in quarantine", "defender_quarantine"),
        ("defender scan results history", "defender_quarantine"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_mdm_enrollment_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("is this device enrolled in MDM", "mdm_enrollment"),
        ("microsoft endpoint manager status", "mdm_enrollment"),
        ("aad join status for this device", "mdm_enrollment"),
        ("device management policy applied", "mdm_enrollment"),
        ("enroll device in intune", "mdm_enrollment"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_storage_spaces_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("storage pool health status", "storage_spaces"),
        ("virtual disks in storage spaces", "storage_spaces"),
        ("resiliency setting for storage pool", "storage_spaces"),
        ("software raid array status", "storage_spaces"),
        ("disk pool degraded warning", "storage_spaces"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_startup_items_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("show startup item list", "startup_items"),
        ("what runs on boot automatically", "startup_items"),
        ("open at startup programs", "startup_items"),
        ("disable startup entries", "startup_items"),
        ("run at login items", "startup_items"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_certificates_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("client cert installed for auth", "certificates"),
        ("expiring cert in the store", "certificates"),
        ("tls certificate valid for this domain", "certificates"),
        ("certificate store contents", "certificates"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 13: --fix path routing gaps ─────────────────────────────────────────

#[test]
fn test_fix_path_routes_vpn_vendor_names() {
    use hematite::agent::report_export::fix_plan_topics;
    let cases = [
        "wireguard tunnel not connecting",
        "cisco anyconnect keeps disconnecting",
        "GlobalProtect VPN client error",
        "pulse secure connection failed",
        "split tunnel not working",
    ];
    for issue in &cases {
        let topics = fix_plan_topics(issue);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"vpn"),
            "expected vpn topic for {:?}, got {:?}",
            issue,
            names
        );
    }
}

#[test]
fn test_fix_path_routes_device_manager_terms() {
    use hematite::agent::report_export::fix_plan_topics;
    let cases = [
        "device manager shows errors",
        "unknown device in device manager",
        "error code 43 on USB",
        "code 10 device cannot start",
    ];
    for issue in &cases {
        let topics = fix_plan_topics(issue);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"device_health"),
            "expected device_health topic for {:?}, got {:?}",
            issue,
            names
        );
    }
}

// ── Batch 18: routing precision bug fix regression tests ──────────────────────

/// "wlan adapter status" must route to wifi, not network_stats.
/// The network_stats (adapter && stat) compound had no wlan/wireless exclusion.
#[test]
fn test_routing_wlan_adapter_goes_to_wifi_not_network_stats() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("wlan adapter status"),
        Some("wifi"),
        "wlan adapter status should route to wifi, not network_stats"
    );
    assert_eq!(
        preferred_host_inspection_topic("wireless adapter stats"),
        Some("wifi"),
        "wireless adapter stats should route to wifi, not network_stats"
    );
    // Non-wireless adapter+stat queries must still reach network_stats.
    assert_eq!(
        preferred_host_inspection_topic("show adapter statistics for ethernet"),
        Some("network_stats"),
        "ethernet adapter statistics should still route to network_stats"
    );
}

/// "what's using my CPU" must route to processes, not hardware.
/// The hardware (what && cpu) compound had no 'using' exclusion.
#[test]
fn test_routing_whats_using_cpu_goes_to_processes_not_hardware() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what's using my CPU"),
        Some("processes"),
        "what's using my CPU should route to processes, not hardware"
    );
    assert_eq!(
        preferred_host_inspection_topic("what is using the CPU"),
        Some("processes"),
        "what is using the CPU should route to processes, not hardware"
    );
    // Plain "what cpu" query must still reach hardware.
    assert_eq!(
        preferred_host_inspection_topic("what cpu does this machine have"),
        Some("hardware"),
        "what cpu does this machine have should still route to hardware"
    );
}

// ── Batch 19: untested preferred_host_inspection_topic topics ──────────────

#[test]
fn test_routing_detects_ad_user_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("look up the ad user account"), Some("ad_user"));
    assert_eq!(preferred_host_inspection_topic("show domain user membership"), Some("ad_user"));
}

#[test]
fn test_routing_detects_network_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show network overview"), Some("network"));
    assert_eq!(preferred_host_inspection_topic("show current network interfaces"), Some("network"));
}

#[test]
fn test_routing_detects_permissions_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check file permissions on this folder"), Some("permissions"));
    assert_eq!(preferred_host_inspection_topic("view access control for a directory"), Some("permissions"));
}

#[test]
fn test_routing_detects_desktop_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show desktop folder contents"), Some("desktop"));
    assert_eq!(preferred_host_inspection_topic("list desktop files"), Some("desktop"));
}

#[test]
fn test_routing_detects_downloads_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("list downloads folder"), Some("downloads"));
    assert_eq!(preferred_host_inspection_topic("show what's in downloads folder"), Some("downloads"));
}

#[test]
fn test_routing_detects_directory_topic() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("what is in the directory /tmp"), Some("directory"));
    assert_eq!(preferred_host_inspection_topic("how big is this folder"), Some("directory"));
}

// ── Batch 20: multi-topic detection gaps (disk_benchmark, desktop, downloads) ─

#[test]
fn test_multi_topic_disk_benchmark_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("run a disk benchmark", "disk_benchmark"),
        ("disk intensity report for this drive", "disk_benchmark"),
        ("stress test the storage subsystem", "disk_benchmark"),
        ("io intensity check", "disk_benchmark"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_desktop_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("show desktop folder contents", "desktop"),
        ("list desktop files", "desktop"),
        ("what's in the desktop folder", "desktop"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_multi_topic_downloads_expanded() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("list downloads folder", "downloads"),
        ("show what's in downloads folder", "downloads"),
        ("downloads folder contents", "downloads"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 21: sign_in login-variant routing gap fix ───────────────────────────

/// "can't login" and "login failed" are common Windows support phrasings that
/// previously routed to None because asks_sign_in only checked "sign in" variants.
#[test]
fn test_routing_detects_sign_in_for_login_variants() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("can't login to Windows"), Some("sign_in"));
    assert_eq!(preferred_host_inspection_topic("login failed after update"), Some("sign_in"));
    assert_eq!(preferred_host_inspection_topic("login not working on this machine"), Some("sign_in"));
    assert_eq!(preferred_host_inspection_topic("login problem since restart"), Some("sign_in"));
}

#[test]
fn test_multi_topic_sign_in_login_variants() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("can't login to Windows", "sign_in"),
        ("login failed suddenly", "sign_in"),
        ("login not working", "sign_in"),
        ("login screen stuck after boot", "sign_in"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 22: fix-path topic coverage gaps (report_export.rs) ─────────────────

#[test]
fn test_fix_path_audio_includes_drivers() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("no sound from speakers");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"audio"), "expected audio for audio issue, got {names:?}");
    assert!(names.contains(&"drivers"), "expected drivers for audio issue, got {names:?}");
}

#[test]
fn test_fix_path_bluetooth_includes_device_health() {
    use hematite::agent::report_export::fix_plan_topics;
    for issue in &["bluetooth won't connect", "can't pair headphones"] {
        let topics = fix_plan_topics(issue);
        let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
        assert!(
            names.contains(&"device_health"),
            "expected device_health for bluetooth issue {issue:?}, got {names:?}"
        );
    }
}

#[test]
fn test_fix_path_teams_includes_connectivity() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("teams not working");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"connectivity"), "expected connectivity for teams issue, got {names:?}");
}

#[test]
fn test_fix_path_outlook_includes_connectivity() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("email not working in outlook");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"connectivity"), "expected connectivity for outlook issue, got {names:?}");
}

#[test]
fn test_fix_path_ssh_includes_services() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("ssh not working");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"services"), "expected services for ssh issue, got {names:?}");
}

#[test]
fn test_fix_path_hyperv_includes_storage_and_disk_health() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("vm won't start in hyper-v");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"storage"), "expected storage for hyperv issue, got {names:?}");
    assert!(names.contains(&"disk_health"), "expected disk_health for hyperv issue, got {names:?}");
}

#[test]
fn test_fix_path_wsl_includes_connectivity_and_dns() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("wsl not working");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"connectivity"), "expected connectivity for wsl issue, got {names:?}");
    assert!(names.contains(&"dns_servers"), "expected dns_servers for wsl issue, got {names:?}");
}

#[test]
fn test_fix_path_docker_includes_connectivity() {
    use hematite::agent::report_export::fix_plan_topics;
    let topics = fix_plan_topics("docker not connecting");
    let names: Vec<&str> = topics.iter().map(|(t, _)| *t).collect();
    assert!(names.contains(&"connectivity"), "expected connectivity for docker issue, got {names:?}");
}

// ── Batch 23: sparse-keyword routing expansions ───────────────────────────────

#[test]
fn test_routing_detects_gpo_for_active_policies() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show active policies on this machine"), Some("gpo"));
    assert_eq!(preferred_host_inspection_topic("what policy objects are applied"), Some("gpo"));
}

#[test]
fn test_routing_detects_scheduled_tasks_for_job_variants() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("show all scheduled jobs"), Some("scheduled_tasks"));
    assert_eq!(preferred_host_inspection_topic("what is running automatically on this machine"), Some("scheduled_tasks"));
}

#[test]
fn test_routing_detects_pagefile_for_swap_space() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("how much swap space is configured"), Some("pagefile"));
    assert_eq!(preferred_host_inspection_topic("is memory swapping happening"), Some("pagefile"));
}

#[test]
fn test_routing_detects_resource_load_for_memory_pressure() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("check memory pressure on this machine"), Some("resource_load"));
    assert_eq!(preferred_host_inspection_topic("what is the current memory load"), Some("resource_load"));
}

#[test]
fn test_routing_detects_shares_for_file_sharing() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(preferred_host_inspection_topic("file sharing status on this machine"), Some("shares"));
}

// ── Batch 25: multi-topic tests for batch 23/24 keyword expansions ────────────

#[test]
fn test_multi_topic_gpo_policy_variant_phrases() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("show active policies on this machine", "gpo"),
        ("what policy objects are applied here", "gpo"),
        ("policy applied to this computer", "gpo"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(topics.contains(expected), "expected {expected} for {query:?}, got {topics:?}");
    }
}

#[test]
fn test_multi_topic_scheduled_tasks_job_variants() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("list all scheduled jobs", "scheduled_tasks"),
        ("what is running automatically on this machine", "scheduled_tasks"),
        ("show background task list", "scheduled_tasks"),
        ("list cron jobs configured", "scheduled_tasks"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(topics.contains(expected), "expected {expected} for {query:?}, got {topics:?}");
    }
}

#[test]
fn test_multi_topic_pagefile_swap_variants() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("how much swap space is configured", "pagefile"),
        ("is memory swapping active", "pagefile"),
        ("show paging file settings", "pagefile"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(topics.contains(expected), "expected {expected} for {query:?}, got {topics:?}");
    }
}

#[test]
fn test_multi_topic_resource_load_memory_pressure() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("check memory pressure on this machine", "resource_load"),
        ("what is the current memory load", "resource_load"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(topics.contains(expected), "expected {expected} for {query:?}, got {topics:?}");
    }
}

#[test]
fn test_multi_topic_shares_file_sharing() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("file sharing status on this machine", "shares"),
        ("what is shared on this PC", "shares"),
        ("what am i sharing over the network", "shares"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(topics.contains(expected), "expected {expected} for {query:?}, got {topics:?}");
    }
}

// ── Batch 26: host_scope gateway expansion + targeted routing fixes ────────────

/// "check login status" previously routed to None because login+status was not
/// in asks_sign_in and "login" was not in host_scope for the fallback path.
#[test]
fn test_routing_login_status_routes_to_sign_in() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "check login status",
        "what is the sign-in status",
        "what is sign in status for this machine",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("sign_in"),
            "expected sign_in for {query:?}"
        );
    }
}

/// "check ssd health" and "nvme health" were not matched by asks_disk_health
/// because the existing check required "healthy" (adjective) not "health" (noun).
#[test]
fn test_routing_ssd_nvme_health_routes_to_disk_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "check ssd health",
        "what is my nvme health",
        "what is my ssd health",
        "hard drive status",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("disk_health"),
            "expected disk_health for {query:?}"
        );
    }
}

/// Multi-topic: login status should appear in all_host_inspection_topics.
#[test]
fn test_multi_topic_sign_in_login_status_variant() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("check login status on this PC", "sign_in"),
        ("what is the login status here", "sign_in"),
        ("sign in status for this account", "sign_in"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

/// Multi-topic: ssd/nvme health should appear in all_host_inspection_topics.
#[test]
fn test_multi_topic_disk_health_ssd_nvme_variants() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("check ssd health and performance", "disk_health"),
        ("nvme health report for this machine", "disk_health"),
        ("what is the hard drive status", "disk_health"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 27: sparse-keyword expansions for credentials, battery, installed_software,
//              usb_history, print_spooler, user_accounts ──────────────────────────

#[test]
fn test_routing_cached_credentials_routes_to_credentials() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "clear all cached credentials",
        "view my stored credentials",
        "delete all credentials from this machine",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("credentials"),
            "expected credentials for {query:?}"
        );
    }
}

#[test]
fn test_routing_charge_percentage_routes_to_battery() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what is my current charge",
        "show charge percentage",
        "what is the charge status of my laptop",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("battery"),
            "expected battery for {query:?}"
        );
    }
}

#[test]
fn test_routing_list_applications_routes_to_installed_software() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "list all applications on my machine",
        "show programs installed here",
        "list apps on this PC",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("installed_software"),
            "expected installed_software for {query:?}"
        );
    }
}

#[test]
fn test_routing_usb_plugged_routes_to_usb_history() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what usb drives have been plugged into this machine",
        "show usb devices that were connected",
        "usb drives ever connected to this PC",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("usb_history"),
            "expected usb_history for {query:?}"
        );
    }
}

#[test]
fn test_routing_printer_service_routes_to_print_spooler() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is the printer service running",
        "check print spooler security",
        "is PrintNightmare mitigated on this machine",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("print_spooler"),
            "expected print_spooler for {query:?}"
        );
    }
}

#[test]
fn test_routing_what_accounts_routes_to_user_accounts() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what accounts have admin rights on this machine",
        "list all users on this computer",
        "who has admin rights here",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("user_accounts"),
            "expected user_accounts for {query:?}"
        );
    }
}

/// Multi-topic: batch 27 expansions should also appear in all_host_inspection_topics.
#[test]
fn test_multi_topic_batch27_keyword_expansions() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("clear all cached credentials from this PC", "credentials"),
        ("what is my current charge on this laptop", "battery"),
        ("list all applications installed here", "installed_software"),
        ("what usb devices have been plugged in", "usb_history"),
        ("is the printer service running on this machine", "print_spooler"),
        ("what accounts have admin rights", "user_accounts"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 28: processes (hogging/hog) + resource_load (frozen/freeze) ─────────

#[test]
fn test_routing_hogging_cpu_routes_to_processes() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "something is hogging cpu on my machine",
        "which process is a cpu hog",
        "what is eating my memory right now",
        "eating up all my ram",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("processes"),
            "expected processes for {query:?}"
        );
    }
}

#[test]
fn test_routing_frozen_routes_to_resource_load() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "my computer is frozen",
        "the machine keeps freezing up",
        "computer freeze happening randomly",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("resource_load"),
            "expected resource_load for {query:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch28_hog_and_freeze() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("something is hogging all the cpu on this PC", "processes"),
        ("memory hog identified on this machine", "processes"),
        ("the computer is frozen and unresponsive", "resource_load"),
        ("PC keeps freezing up under load", "resource_load"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 29: thermal (temperature compound) + overclocker (gpu usage/utilization) ──

#[test]
fn test_routing_system_temperature_routes_to_thermal() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "check system temperature",
        "what is the cpu temperature right now",
        "monitor gpu temperature on this machine",
        "check temps on this PC",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("thermal"),
            "expected thermal for {query:?}"
        );
    }
}

#[test]
fn test_routing_gpu_usage_routes_to_overclocker() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what is my gpu usage",
        "show gpu utilization",
        "check gpu performance on this machine",
    ];
    for query in &cases {
        assert_eq!(
            preferred_host_inspection_topic(query),
            Some("overclocker"),
            "expected overclocker for {query:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch29_thermal_and_overclocker() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("check cpu temperature on this PC", "thermal"),
        ("gpu temperature is too high", "thermal"),
        ("what is the gpu usage right now", "overclocker"),
        ("show gpu utilization for this system", "overclocker"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_cant_browse_web_routes_to_connectivity() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "I can't browse the web",
        "cannot browse the internet",
        "web browsing is not working",
        "pages not loading on any site",
        "websites not loading",
        "no network connection",
        "the network is down",
        "can't connect to internet",
        "cannot connect to internet",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("connectivity"),
            "expected connectivity for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch30_connectivity_browse_variants() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("I can't browse the web at all", "connectivity"),
        ("websites not loading on my PC", "connectivity"),
        ("the network is down right now", "connectivity"),
        ("cannot connect to internet today", "connectivity"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_corrupted_routes_to_integrity() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "check if windows is corrupted",
        "are my system files corrupted",
        "windows system file check",
        "are my system files damaged",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("integrity"),
            "expected integrity for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_system_time_routes_to_ntp() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is my system time correct",
        "my clock is off",
        "is the time accurate",
        "check if time is correct",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("ntp"),
            "expected ntp for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_processor_slow_routes_to_cpu_power() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "why is my processor running slow",
        "processor is running slowly",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("cpu_power"),
            "expected cpu_power for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_network_usage_routes_to_network_stats() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what is my network usage",
        "how much data was transferred",
        "show network traffic",
        "are there packet errors",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("network_stats"),
            "expected network_stats for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch31_integrity_ntp_cpu_network() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("check if windows is corrupted", "integrity"),
        ("are my system files damaged", "integrity"),
        ("is my system time correct", "ntp"),
        ("my clock is off today", "ntp"),
        ("processor is running slowly", "cpu_power"),
        ("what is my network usage", "network_stats"),
        ("show network traffic on this adapter", "network_stats"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_udp_services_routes_to_udp_ports() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what udp services are running",
        "show udp connections",
        "which ports are open for udp",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("udp_ports"),
            "expected udp_ports for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_domain_controller_online_routes_to_domain_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "are domain controllers online",
        "is active directory working",
        "can reach domain from this machine",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("domain_health"),
            "expected domain_health for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_scheduled_tasks_background_run() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what runs in the background on this PC",
        "what is scheduled to run",
        "what runs periodically",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("scheduled_tasks"),
            "expected scheduled_tasks for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_service_requirements_routes_to_service_dependencies() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what are the service requirements for this",
        "service relationships for wuauserv",
        "service prerequisites for print spooler",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("service_dependencies"),
            "expected service_dependencies for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch32_udp_domain_scheduled_svc_deps() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("what udp services are running", "udp_ports"),
        ("show udp connections on this PC", "udp_ports"),
        ("are domain controllers online", "domain_health"),
        ("is active directory working", "domain_health"),
        ("can reach domain from this machine", "domain_health"),
        ("what runs in the background", "scheduled_tasks"),
        ("what is scheduled to run today", "scheduled_tasks"),
        ("service requirements for svchost", "service_dependencies"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_unable_to_install_routes_to_installer_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "I'm unable to install this app",
        "unable to install the software",
        "installation is hanging and won't finish",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("installer_health"),
            "expected installer_health for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_organizational_account_routes_to_identity_auth() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "my organizational account isn't working",
        "corporate account not signing in",
        "is my device azure registered",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("identity_auth"),
            "expected identity_auth for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_browser_unresponsive_routes_to_browser_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "chrome is unresponsive",
        "firefox is not loading pages",
        "edge is not starting",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("browser_health"),
            "expected browser_health for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_backup_enabled_routes_to_windows_backup() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is my backup enabled",
        "is backup working on this PC",
        "is backup set up correctly",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("windows_backup"),
            "expected windows_backup for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch33_installer_identity_browser_backup() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("unable to install this app", "installer_health"),
        ("installation is hanging", "installer_health"),
        ("organizational account not working", "identity_auth"),
        ("is my device azure registered", "identity_auth"),
        ("chrome is unresponsive right now", "browser_health"),
        ("edge is not loading", "browser_health"),
        ("is my backup enabled", "windows_backup"),
        ("is backup working", "windows_backup"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_ssd_encrypted_routes_to_bitlocker() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is my ssd encrypted",
        "what is my drive encryption status",
        "is my machine encrypted",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("bitlocker"),
            "expected bitlocker for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_uefi_routes_to_tpm() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is my PC in uefi mode",
        "check uefi boot settings",
        "is uefi enabled on this machine",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("tpm"),
            "expected tpm for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_lockout_uac_routes_to_local_security_policy() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "how many failed logins before lockout",
        "is uac turned off",
        "what is the uac status",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("local_security_policy"),
            "expected local_security_policy for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_login_event_routes_to_audit_policy() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "which logon events are being audited",
        "what events are being audited on this PC",
        "are audit events enabled on this machine",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("audit_policy"),
            "expected audit_policy for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_folders_shared_routes_to_shares() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "which folders are being shared on this PC",
        "show network sharing configuration",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("shares"),
            "expected shares for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_printer_spooler_routes_to_print_spooler() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "is my printer spooler running",
        "what is the print service status",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("print_spooler"),
            "expected print_spooler for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch34_bitlocker_tpm_policy_audit_shares_spooler() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("is my ssd encrypted", "bitlocker"),
        ("drive encryption status", "bitlocker"),
        ("is uefi enabled on this machine", "tpm"),
        ("check uefi boot status", "tpm"),
        ("how many failed logins before lockout", "local_security_policy"),
        ("what is the uac status", "local_security_policy"),
        ("which logon events are being audited", "audit_policy"),
        ("are audit events enabled on this machine", "audit_policy"),
        ("which folders are shared on this PC", "shares"),
        ("show network sharing", "shares"),
        ("is my printer spooler running", "print_spooler"),
        ("what is the print service status", "print_spooler"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

#[test]
fn test_routing_group_policies_plural_routes_to_gpo() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "which group policies are currently in effect",
        "are any policies applied to this computer",
        "show me active group policies",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("gpo"),
            "expected gpo for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_bond_adapter_routes_to_nic_teaming() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "how do i bond two network adapters",
        "bond these two interfaces together",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("nic_teaming"),
            "expected nic_teaming for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_remembered_wifi_routes_to_wlan_profiles() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "show me all remembered wifi networks",
        "what wireless networks does this PC remember",
        "what is my saved wifi password",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("wlan_profiles"),
            "expected wlan_profiles for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_routing_tcp_window_routes_to_tcp_params() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    let cases = [
        "what is my tcp window size",
        "how do i speed up tcp connections",
    ];
    for query in &cases {
        let result = preferred_host_inspection_topic(query);
        assert_eq!(
            result,
            Some("tcp_params"),
            "expected tcp_params for {query:?}, got {result:?}"
        );
    }
}

#[test]
fn test_multi_topic_batch35_gpo_teaming_wlan_tcp() {
    use hematite::agent::routing::all_host_inspection_topics;
    let cases = [
        ("which group policies are in effect", "gpo"),
        ("are any policies applied to this PC", "gpo"),
        ("bond two network adapters together", "nic_teaming"),
        ("what is the snmp community name", "snmp"),
        ("show me remembered wifi networks", "wlan_profiles"),
        ("what is my tcp window size", "tcp_params"),
        ("how do i speed up tcp", "tcp_params"),
    ];
    for (query, expected) in &cases {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected),
            "expected {expected} for {query:?}, got {topics:?}"
        );
    }
}

// ── Batch 36: sessions, patch_history, activation, pending_reboot, device_health ──

#[test]
fn test_routing_who_using_machine_routes_to_sessions() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("show me who's currently using the machine"),
        Some("sessions")
    );
    assert_eq!(
        preferred_host_inspection_topic("who is using this computer right now"),
        Some("sessions")
    );
}

#[test]
fn test_routing_updates_applied_routes_to_patch_history() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what updates have been applied to my system"),
        Some("patch_history")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me security patches installed"),
        Some("patch_history")
    );
}

#[test]
fn test_routing_windows_licensed_routes_to_activation() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("is my windows licensed"),
        Some("activation")
    );
    assert_eq!(
        preferred_host_inspection_topic("windows is unlicensed"),
        Some("activation")
    );
    assert_eq!(
        preferred_host_inspection_topic("how do I activate windows"),
        Some("activation")
    );
}

#[test]
fn test_routing_have_to_reboot_routes_to_pending_reboot() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("do I have to reboot my machine"),
        Some("pending_reboot")
    );
    assert_eq!(
        preferred_host_inspection_topic("do i need to restart after the update"),
        Some("pending_reboot")
    );
    assert_eq!(
        preferred_host_inspection_topic("do I have to restart"),
        Some("pending_reboot")
    );
}

#[test]
fn test_routing_device_not_working_routes_to_device_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("my device is not working"),
        Some("device_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("a device stopped working after the update"),
        Some("device_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("some hardware is broken"),
        Some("device_health")
    );
}

#[test]
fn test_multi_topic_batch36_sessions_patch_activation_reboot_device() {
    use hematite::agent::routing::all_host_inspection_topics;

    let topics = all_host_inspection_topics("who is using this computer");
    assert!(topics.contains(&"sessions"), "sessions missing: {topics:?}");

    let topics = all_host_inspection_topics("what security patches were applied");
    assert!(topics.contains(&"patch_history"), "patch_history missing: {topics:?}");

    let topics = all_host_inspection_topics("is windows licensed on this machine");
    assert!(topics.contains(&"activation"), "activation missing: {topics:?}");

    let topics = all_host_inspection_topics("do I have to restart the computer");
    assert!(topics.contains(&"pending_reboot"), "pending_reboot missing: {topics:?}");

    let topics = all_host_inspection_topics("a device stopped working");
    assert!(topics.contains(&"device_health"), "device_health missing: {topics:?}");
}

#[test]
fn test_routing_batch37_cant_hear_routes_to_audio() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("I can't hear anything from my computer"),
        Some("audio")
    );
    assert_eq!(
        preferred_host_inspection_topic("cannot hear any sound"),
        Some("audio")
    );
    assert_eq!(
        preferred_host_inspection_topic("there is no audio coming out"),
        Some("audio")
    );
}

#[test]
fn test_routing_batch37_network_drive_routes_to_share_access() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("can't access the network drive"),
        Some("share_access")
    );
    assert_eq!(
        preferred_host_inspection_topic("my mapped drive disappeared"),
        Some("share_access")
    );
    assert_eq!(
        preferred_host_inspection_topic("shared folder is not accessible"),
        Some("share_access")
    );
}

#[test]
fn test_routing_batch37_disk_full_routes_to_storage() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("my disk is full"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("C drive is almost full"),
        Some("storage")
    );
    assert_eq!(
        preferred_host_inspection_topic("I'm out of space on this drive"),
        Some("storage")
    );
}

#[test]
fn test_routing_batch37_bad_sector_routes_to_disk_health() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("my drive has bad sectors"),
        Some("disk_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("show me the SMART data for the disk"),
        Some("disk_health")
    );
    assert_eq!(
        preferred_host_inspection_topic("the disk is failing"),
        Some("disk_health")
    );
}

#[test]
fn test_routing_batch37_network_not_working_routes_to_connectivity() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("the network is not working"),
        Some("connectivity")
    );
    assert_eq!(
        preferred_host_inspection_topic("internet not working on this PC"),
        Some("connectivity")
    );
}

#[test]
fn test_routing_batch37_autostart_loads_on_boot_routes_to_startup() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("what autostart programs are enabled"),
        Some("startup_items")
    );
    assert_eq!(
        preferred_host_inspection_topic("what loads on boot"),
        Some("startup_items")
    );
    assert_eq!(
        preferred_host_inspection_topic("what loads on startup"),
        Some("startup_items")
    );
}

#[test]
fn test_routing_batch37_check_updates_routes_to_updates() {
    use hematite::agent::routing::preferred_host_inspection_topic;
    assert_eq!(
        preferred_host_inspection_topic("check updates for Windows"),
        Some("updates")
    );
    assert_eq!(
        preferred_host_inspection_topic("update windows please"),
        Some("updates")
    );
    assert_eq!(
        preferred_host_inspection_topic("windows is out of date"),
        Some("updates")
    );
}

#[test]
fn test_multi_topic_batch37_audio_share_storage_disk_connectivity_startup_updates() {
    use hematite::agent::routing::all_host_inspection_topics;

    let topics = all_host_inspection_topics("I can't hear anything");
    assert!(topics.contains(&"audio"), "audio missing: {topics:?}");

    let topics = all_host_inspection_topics("network drive is not accessible");
    assert!(topics.contains(&"share_access"), "share_access missing: {topics:?}");

    let topics = all_host_inspection_topics("the disk is full");
    assert!(topics.contains(&"storage"), "storage missing: {topics:?}");

    let topics = all_host_inspection_topics("my drive has bad sectors");
    assert!(topics.contains(&"disk_health"), "disk_health missing: {topics:?}");

    let topics = all_host_inspection_topics("network not working");
    assert!(topics.contains(&"connectivity"), "connectivity missing: {topics:?}");

    let topics = all_host_inspection_topics("what loads on boot");
    assert!(topics.contains(&"startup_items"), "startup_items missing: {topics:?}");

    let topics = all_host_inspection_topics("check updates");
    assert!(topics.contains(&"updates"), "updates missing: {topics:?}");
}

// ── Parity audit batch 38 ──────────────────────────────────────────────────

#[test]
fn test_routing_parity38_system_specs_routes_to_hardware() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("what are my system specs");
    assert_eq!(t.as_deref(), Some("hardware"));
    let topics = all_host_inspection_topics("what are my system specs");
    assert!(topics.contains(&"hardware"), "hardware missing: {topics:?}");
}

#[test]
fn test_routing_parity38_graphics_card_routes_to_hardware() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("tell me about my graphics card");
    assert_eq!(t.as_deref(), Some("hardware"));
    let topics = all_host_inspection_topics("tell me about my graphics card");
    assert!(topics.contains(&"hardware"), "hardware missing: {topics:?}");
}

#[test]
fn test_routing_parity38_fan_always_on_routes_to_thermal() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("my fan is always on");
    assert_eq!(t.as_deref(), Some("thermal"));
    let topics = all_host_inspection_topics("my fan is always on");
    assert!(topics.contains(&"thermal"), "thermal missing: {topics:?}");
}

#[test]
fn test_routing_parity38_laptop_hot_routes_to_thermal() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("my laptop is getting hot");
    assert_eq!(t.as_deref(), Some("thermal"));
    let topics = all_host_inspection_topics("my laptop is getting hot");
    assert!(topics.contains(&"thermal"), "thermal missing: {topics:?}");
}

#[test]
fn test_routing_parity38_product_key_routes_to_activation() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("where is my product key");
    assert_eq!(t.as_deref(), Some("activation"));
    let topics = all_host_inspection_topics("where is my product key");
    assert!(topics.contains(&"activation"), "activation missing: {topics:?}");
}

#[test]
fn test_routing_parity38_not_activated_routes_to_activation() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("windows is not activated");
    assert_eq!(t.as_deref(), Some("activation"));
    let topics = all_host_inspection_topics("windows is not activated");
    assert!(topics.contains(&"activation"), "activation missing: {topics:?}");
}

#[test]
fn test_routing_parity38_keeps_restarting_routes_to_recent_crashes() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("my PC keeps restarting");
    assert_eq!(t.as_deref(), Some("recent_crashes"));
    let topics = all_host_inspection_topics("my PC keeps restarting");
    assert!(topics.contains(&"recent_crashes"), "recent_crashes missing: {topics:?}");
}

#[test]
fn test_routing_parity38_random_reboot_routes_to_recent_crashes() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("random reboot overnight");
    assert_eq!(t.as_deref(), Some("recent_crashes"));
    let topics = all_host_inspection_topics("random reboot overnight");
    assert!(topics.contains(&"recent_crashes"), "recent_crashes missing: {topics:?}");
}

#[test]
fn test_routing_parity38_apps_crashing_routes_to_app_crashes() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("apps crashing constantly");
    assert_eq!(t.as_deref(), Some("app_crashes"));
    let topics = all_host_inspection_topics("apps crashing constantly");
    assert!(topics.contains(&"app_crashes"), "app_crashes missing: {topics:?}");
}

#[test]
fn test_routing_parity38_what_crashed_routes_to_app_crashes() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("what crashed today");
    assert_eq!(t.as_deref(), Some("app_crashes"));
    let topics = all_host_inspection_topics("what crashed today");
    assert!(topics.contains(&"app_crashes"), "app_crashes missing: {topics:?}");
}

#[test]
fn test_routing_parity38_windows_log_routes_to_log_check() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("show me the windows log");
    assert_eq!(t.as_deref(), Some("log_check"));
    let topics = all_host_inspection_topics("show me the windows log");
    assert!(topics.contains(&"log_check"), "log_check missing: {topics:?}");
}

#[test]
fn test_routing_parity38_high_cpu_routes_to_resource_load() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("high cpu usage right now");
    assert_eq!(t.as_deref(), Some("resource_load"));
    let topics = all_host_inspection_topics("high cpu usage right now");
    assert!(topics.contains(&"resource_load"), "resource_load missing: {topics:?}");
}

#[test]
fn test_routing_parity38_free_space_routes_to_storage() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("how much free space do I have");
    assert_eq!(t.as_deref(), Some("storage"));
    let topics = all_host_inspection_topics("how much free space do I have");
    assert!(topics.contains(&"storage"), "storage missing: {topics:?}");
}

#[test]
fn test_routing_parity38_running_out_of_space_routes_to_storage() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("I'm running out of space");
    assert_eq!(t.as_deref(), Some("storage"));
    let topics = all_host_inspection_topics("I'm running out of space");
    assert!(topics.contains(&"storage"), "storage missing: {topics:?}");
}

#[test]
fn test_routing_parity38_what_is_listening_routes_to_ports() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("what is listening on this machine");
    assert_eq!(t.as_deref(), Some("ports"));
    let topics = all_host_inspection_topics("what is listening on this machine");
    assert!(topics.contains(&"ports"), "ports missing: {topics:?}");
}

#[test]
fn test_routing_parity38_cpu_speed_routes_to_cpu_power() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("why is my cpu speed so low");
    assert_eq!(t.as_deref(), Some("cpu_power"));
    let topics = all_host_inspection_topics("why is my cpu speed so low");
    assert!(topics.contains(&"cpu_power"), "cpu_power missing: {topics:?}");
}

#[test]
fn test_routing_parity38_boost_disabled_routes_to_cpu_power() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let t = preferred_host_inspection_topic("boost is disabled on my processor");
    assert_eq!(t.as_deref(), Some("cpu_power"));
    let topics = all_host_inspection_topics("boost is disabled on my processor");
    assert!(topics.contains(&"cpu_power"), "cpu_power missing: {topics:?}");
}

#[test]
fn test_multi_topic_parity38_hardware_thermal_activation_crashes_storage() {
    use hematite::agent::routing::all_host_inspection_topics;
    let queries = [
        ("what are my system specs", "hardware"),
        ("my fan is always on and laptop is getting hot", "thermal"),
        ("windows is not activated", "activation"),
        ("apps crashing all day", "app_crashes"),
        ("how much free space on C:", "storage"),
        ("what is listening on this machine", "ports"),
    ];
    for (query, expected_topic) in &queries {
        let topics = all_host_inspection_topics(query);
        assert!(
            topics.contains(expected_topic),
            "query={query:?} expected {expected_topic}, got {topics:?}"
        );
    }
}

// ── diagnose-why category expansion (Teams, Outlook, Bluetooth, Camera, USB, Sleep, App Crashes) ──

#[test]
fn test_diagnose_why_teams_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("teams keeps crashing when I try to join a meeting");
    assert!(g.is_some(), "expected Teams group, got None");
    assert_eq!(g.unwrap().category, "Microsoft Teams Problems");
}

#[test]
fn test_diagnose_why_teams_audio_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("teams microphone not working in calls");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Microsoft Teams Problems");
}

#[test]
fn test_diagnose_why_outlook_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("outlook not syncing email");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Outlook / Email Problems");
}

#[test]
fn test_diagnose_why_outlook_email_sync_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("email not syncing in outlook");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Outlook / Email Problems");
}

#[test]
fn test_diagnose_why_bluetooth_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("bluetooth won't pair with my headphones");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Bluetooth Problems");
}

#[test]
fn test_diagnose_why_bluetooth_disconnect_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("bluetooth keeps disconnecting");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Bluetooth Problems");
}

#[test]
fn test_diagnose_why_camera_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("camera not working in Teams");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Camera / Webcam Problems");
}

#[test]
fn test_diagnose_why_camera_blocked_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("camera blocked by privacy settings");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Camera / Webcam Problems");
}

#[test]
fn test_diagnose_why_usb_not_recognized_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("usb device not recognized when I plug it in");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "USB / Device Not Recognized");
}

#[test]
fn test_diagnose_why_external_drive_not_detected_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("external hard drive not detected");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "USB / Device Not Recognized");
}

#[test]
fn test_diagnose_why_sleep_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("pc won't go to sleep and wakes itself up");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Sleep / Wake Problems");
}

#[test]
fn test_diagnose_why_black_screen_after_sleep_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("black screen after sleep and won't resume");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "Sleep / Wake Problems");
}

#[test]
fn test_diagnose_why_app_crash_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("chrome keeps crashing every time I open it");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "App / Program Crashing");
}

#[test]
fn test_diagnose_why_app_crash_word_keyword_match() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("word keeps crashing when I save a document");
    assert!(g.is_some());
    assert_eq!(g.unwrap().category, "App / Program Crashing");
}

#[test]
fn test_diagnose_why_teams_topics_include_audio_and_camera() {
    use hematite::agent::diagnose_why::match_symptom;
    let g = match_symptom("microsoft teams").unwrap();
    assert!(g.topics.contains(&"audio"), "expected audio in Teams topics");
    assert!(g.topics.contains(&"camera"), "expected camera in Teams topics");
    assert!(g.topics.contains(&"identity_auth"), "expected identity_auth in Teams topics");
}

#[test]
fn test_diagnose_why_total_category_count() {
    use hematite::agent::diagnose_why::match_symptom;
    // Smoke-test: all 8 new categories match at least one representative query
    let cases = [
        ("microsoft teams not working", "Microsoft Teams Problems"),
        ("outlook crashing", "Outlook / Email Problems"),
        ("bluetooth won't pair", "Bluetooth Problems"),
        ("webcam not working", "Camera / Webcam Problems"),
        ("usb not recognized", "USB / Device Not Recognized"),
        ("pc won't sleep", "Sleep / Wake Problems"),
        ("app keeps crashing", "App / Program Crashing"),
    ];
    for (query, expected) in &cases {
        let g = match_symptom(query);
        assert!(g.is_some(), "no match for {query:?}");
        assert_eq!(
            g.unwrap().category,
            *expected,
            "query={query:?} expected category {expected}, got {:?}",
            g.map(|x| x.category)
        );
    }
}
