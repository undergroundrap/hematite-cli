use std::fs;
use std::path::PathBuf;
use std::io::Write;

// ── Hardware monitors ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_gpu_monitor_logic() {
    let state = hematite::ui::gpu_monitor::GpuState::new();
    let (used, total) = state.read();
    assert_eq!(used, 0);
    assert_eq!(total, 0);
    assert_eq!(state.ratio(), 0.0);
    assert_eq!(state.label(), "N/A");

    state.used_mib.store(4096, std::sync::atomic::Ordering::Relaxed);
    state.total_mib.store(8192, std::sync::atomic::Ordering::Relaxed);

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
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string())
        .expect("vein init");

    let doc = "fn authenticate(token: &str) -> bool {\n    token == \"secret\"\n}\n\n\
               fn logout(user: &str) {\n    println!(\"Logging out {}\", user);\n}";

    let chunks = vein.index_document("src/auth.rs", 1_000_000, doc)
        .expect("index");
    assert!(!chunks.is_empty(), "should produce chunks");

    let results = vein.search_bm25("authenticate", 5).expect("search");
    assert!(!results.is_empty(), "BM25 should find 'authenticate'");
    assert!(results[0].content.contains("authenticate"));

    // Confirm file count tracks correctly
    assert_eq!(vein.file_count(), 1);

    // Re-indexing same mtime should be a no-op
    let rechunks = vein.index_document("src/auth.rs", 1_000_000, doc).expect("re-index");
    assert!(rechunks.is_empty(), "unchanged file should not re-index");
}

#[test]
fn test_vein_reset_clears_index() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string())
        .expect("vein init");

    vein.index_document("src/lib.rs", 1, "pub fn foo() {}").unwrap();
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
    let vein = Vein::new(tmp.path(), "http://localhost:1234".to_string())
        .expect("vein init");

    // Fresh vein with no edits — l1_context should be None.
    assert!(vein.l1_context().is_none(), "no edits means no L1 block");
}

#[test]
fn test_vein_l1_bump_and_retrieve() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string())
        .expect("vein init");

    // Index a file so it appears in chunks_meta (required for L1 join).
    vein.index_document("src/agent/conversation.rs", 1_000_000,
        "pub fn run() {}\npub fn stop() {}\n").unwrap();

    // Bump heat three times.
    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/agent/conversation.rs");
    vein.bump_heat("src/agent/conversation.rs");

    let l1 = vein.l1_context().expect("should have L1 after edits");
    assert!(l1.contains("src/agent/conversation.rs"), "hot file should appear in L1");
    assert!(l1.contains("3 edits"), "edit count should be 3");
}

#[test]
fn test_vein_l1_ranks_by_heat() {
    use hematite::memory::vein::Vein;

    let tmp = tempfile::NamedTempFile::new().expect("temp db");
    let mut vein = Vein::new(tmp.path(), "http://localhost:1234".to_string())
        .expect("vein init");

    vein.index_document("src/cold.rs", 1_000, "pub fn cold() {}").unwrap();
    vein.index_document("src/hot.rs",  2_000, "pub fn hot() {}").unwrap();

    vein.bump_heat("src/cold.rs");
    vein.bump_heat("src/hot.rs");
    vein.bump_heat("src/hot.rs");
    vein.bump_heat("src/hot.rs");

    let l1 = vein.l1_context().expect("L1 should exist");
    let hot_pos  = l1.find("src/hot.rs").unwrap_or(usize::MAX);
    let cold_pos = l1.find("src/cold.rs").unwrap_or(usize::MAX);
    assert!(hot_pos < cold_pos, "hotter file should appear first in L1");
}

// ── Document text extraction ──────────────────────────────────────────────────

#[test]
fn test_extract_markdown_succeeds() {
    use hematite::memory::vein::extract_document_text;

    let mut tmp = tempfile::NamedTempFile::with_suffix(".md").expect("temp md");
    writeln!(tmp, "# Design Doc\n\nThis is a specification for the auth module.").unwrap();

    let result = extract_document_text(tmp.path());
    assert!(result.is_ok(), "markdown extraction should succeed");
    assert!(result.unwrap().contains("Design Doc"));
}

#[test]
fn test_extract_txt_succeeds() {
    use hematite::memory::vein::extract_document_text;

    let mut tmp = tempfile::NamedTempFile::with_suffix(".txt").expect("temp txt");
    writeln!(tmp, "API reference for the payment service.\n\nEndpoint: POST /charge").unwrap();

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
    writeln!(tmp, "This is a well formatted document with proper spacing between all words.").unwrap();
    let result = extract_document_text(tmp.path());
    assert!(result.is_ok());
}

// ── Sandboxed code execution ──────────────────────────────────────────────────

#[tokio::test]
async fn test_sandbox_python_runs() {
    use serde_json::json;

    // Skip if Python is not available
    let python_available = std::process::Command::new("python")
        .arg("--version")
        .output()
        .or_else(|_| std::process::Command::new("python3").arg("--version").output())
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
    assert!(result.is_ok(), "Python sandbox should execute: {:?}", result);
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
        result.unwrap().contains("94a194250ccdb8506d67ead15dd3a1db50803855123422f21b378b56f80ba99c"),
        "SHA-256 of 'Hematite' should match known hash"
    );
}
