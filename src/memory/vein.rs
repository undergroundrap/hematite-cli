use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// "The Vein" — local RAG memory engine backed by SQLite FTS5 + semantic embeddings.
///
/// Two retrieval modes, used together:
///
/// **BM25 (always available)**
/// Full-text search via SQLite FTS5 with Porter-stemming. Fast, zero extra GPU cost,
/// works as the fallback when the embedding model isn't loaded.
///
/// **Semantic (when LM Studio has an embedding model loaded)**
/// Calls `/v1/embeddings` (nomic-embed-text-v1.5 or similar) to produce 768-dim float
/// vectors for each chunk. At search time the query is embedded and cosine similarity
/// selects the most conceptually relevant chunks — even when no keywords match.
///
/// Hybrid search runs BM25 and semantic in parallel, deduplicates by path, and returns
/// the top-k results ranked by combined score. Semantic results score higher when the
/// embedding model is available; BM25 fills the gap when it isn't.
///
/// Indexing is incremental: files are re-indexed only when their mtime changes. Embedding
/// vectors are stored in a separate `chunks_vec` SQLite table so they survive re-runs
/// without hitting the embedding API again.
pub struct Vein {
    db: std::sync::Arc<std::sync::Mutex<Connection>>,
    /// Base URL of the LLM provider, used for the embeddings endpoint.
    base_url: String,
}

// SAFETY: rusqlite::Connection is !Send by default, but we wrap it in Arc<Mutex>
// and ensure all accesses are serialized by the mutex.
unsafe impl Send for Vein {}
unsafe impl Sync for Vein {}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub content: String,
    /// Combined relevance score (higher = more relevant).
    pub score: f32,
    /// Subsystem room derived from the file path (e.g. "agent", "ui", "tools").
    pub room: String,
    /// Last-modified timestamp from chunks_meta (unix seconds).
    pub last_modified: i64,
    /// Semantic memory type tagged at index time: "decision", "problem",
    /// "milestone", "preference", or "" for unclassified/source/doc chunks.
    pub memory_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VeinHotFile {
    pub path: String,
    pub heat: i64,
    pub last_modified: i64,
    pub room: String,
}

#[derive(Debug, Clone)]
pub struct VeinInspectionSnapshot {
    pub indexed_source_files: usize,
    pub indexed_docs: usize,
    pub indexed_session_exchanges: usize,
    pub embedded_source_doc_chunks: usize,
    pub has_any_embeddings: bool,
    pub active_room: Option<String>,
    pub hot_files: Vec<VeinHotFile>,
    pub l1_ready: bool,
}

#[derive(Debug, Default)]
struct QuerySignals {
    exact_phrases: Vec<String>,
    standout_terms: Vec<String>,
    historical_memory_hint: bool,
    temporal_reference: Option<TemporalReference>,
    /// Memory type the query is asking about — boosts matching chunks.
    /// e.g. "what did we decide" → "decision", "what was the bug" → "problem"
    query_memory_type: Option<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct TemporalReference {
    target_ts: i64,
    window_secs: i64,
}

#[derive(Debug, Deserialize)]
struct SessionReport {
    #[serde(default)]
    session_start: String,
    #[serde(default)]
    transcript: Vec<SessionTranscriptEntry>,
}

#[derive(Debug, Deserialize)]
struct SessionTranscriptEntry {
    #[serde(default)]
    speaker: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug)]
struct SessionExchange {
    path: String,
    last_modified: i64,
    content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionSpeakerKind {
    User,
    Assistant,
    Ignore,
}

/// Derive a subsystem room label from a file path.
/// Uses path segments, filenames, and repo-role hints to map files into
/// stable subsystem rooms. Falls back to the first directory component or
/// "root" when no stronger signal exists.
pub fn detect_room(path: &str) -> String {
    let lower = path.to_lowercase().replace('\\', "/");
    let filename = lower.rsplit('/').next().unwrap_or(&lower);
    let ext = filename.rsplit('.').next().unwrap_or("");

    let mut best_room = None::<&str>;
    let mut best_score = 0i32;
    let mut consider = |room: &'static str, score: i32| {
        if score > best_score {
            best_score = score;
            best_room = Some(room);
        }
    };

    let is_component = |segment: &str| {
        lower == segment
            || lower.starts_with(&format!("{segment}/"))
            || lower.contains(&format!("/{segment}/"))
    };

    if lower.starts_with("session/")
        || lower.starts_with(".hematite/reports/")
        || lower.starts_with(".hematite/imports/")
        || is_component("reports")
        || is_component("imports")
    {
        consider("session", 100);
    }

    if lower.starts_with(".hematite/docs/")
        || is_component("docs")
        || matches!(filename, "readme.md" | "claude.md" | ".hematite.md")
        || matches!(ext, "md" | "markdown" | "pdf" | "rst")
    {
        consider("docs", 80);
    }

    if is_component("tests")
        || filename.contains("diagnostic")
        || filename.ends_with("_test.rs")
        || filename.ends_with(".test.ts")
    {
        consider("tests", 85);
    }

    if lower.starts_with(".github/workflows/")
        || is_component("workflows")
        || filename == ".pre-commit-config.yaml"
        || filename == ".pre-commit-config.yml"
        || filename.contains("hook")
    {
        consider("automation", 84);
    }

    if lower.starts_with("installer/")
        || lower.starts_with("dist/")
        || lower.starts_with("scripts/package-")
        || filename.contains("release")
        || filename.contains("bump-version")
        || ext == "iss"
    {
        consider("release", 82);
    }

    if matches!(
        filename,
        "cargo.toml"
            | "cargo.lock"
            | "package.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "bun.lock"
            | "bun.lockb"
            | "pyproject.toml"
            | "setup.py"
            | "go.mod"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "cmakelists.txt"
            | ".gitignore"
            | "settings.json"
            | "mcp_servers.json"
    ) || filename.ends_with(".sln")
        || filename.ends_with(".csproj")
        || filename.contains("config")
    {
        consider("config", 76);
    }

    if is_component("ui")
        || matches!(
            filename,
            "tui.rs" | "voice.rs" | "hatch.rs" | "gpu_monitor.rs"
        )
    {
        consider("ui", 70);
    }

    if is_component("memory") || matches!(filename, "vein.rs" | "deep_reflect.rs") {
        consider("memory", 72);
    }

    if is_component("tools")
        || matches!(
            filename,
            "verify_build.rs"
                | "host_inspect.rs"
                | "shell.rs"
                | "code_sandbox.rs"
                | "project_map.rs"
                | "runtime_trace.rs"
        )
    {
        consider("tools", 68);
    }

    if filename.contains("mcp")
        || filename.contains("lsp")
        || lower.contains("/mcp/")
        || lower.contains("/lsp/")
    {
        consider("integration", 67);
    }

    if matches!(filename, "main.rs" | "runtime.rs" | "inference.rs")
        || filename.contains("startup")
        || filename.contains("runtime")
    {
        consider("runtime", 66);
    }

    if is_component("agent") {
        consider("agent", 60);
    }

    if lower.starts_with("libs/") || is_component("libs") {
        consider("libs", 58);
    }

    if lower.starts_with("scripts/") || is_component("scripts") {
        consider("scripts", 55);
    }

    if let Some(room) = best_room {
        return room.to_string();
    }

    // Fall back to first directory component
    lower
        .split('/')
        .next()
        .filter(|s| !s.is_empty() && !s.contains('.'))
        .unwrap_or("root")
        .to_string()
}

/// Classify session memory text into a semantic type using zero-cost regex patterns.
/// Returns one of: "decision", "problem", "milestone", "preference", or "" (unclassified).
///
/// Applied only to session/import chunks at index time. Source and doc chunks always get "".
/// Used by reranking to boost chunks whose type matches the query's implied intent.
pub fn detect_memory_type(text: &str) -> &'static str {
    let lower = text.to_lowercase();

    // Decision markers — architectural choices, agreed approaches, "let's use X"
    let decision_patterns = [
        "let's use ",
        "we'll use ",
        "decided to ",
        "going with ",
        "we agreed ",
        "the plan is",
        "we're going to",
        "switching to",
        "we chose",
        "final decision",
        "we settled on",
        "agreed on",
        "we decided",
    ];
    for pat in &decision_patterns {
        if lower.contains(pat) {
            return "decision";
        }
    }

    // Problem markers — bugs, errors, failures, blockers
    let problem_patterns = [
        "bug fixed",
        "bug was",
        "the issue was",
        "root cause",
        "error was",
        "turned out to be",
        "the fix was",
        "was caused by",
        "broken because",
        "fixed by",
        "the problem was",
        "found the bug",
        "port conflict",
        "crash",
        "panicked",
        "segfault",
        "oom",
        "out of memory",
    ];
    for pat in &problem_patterns {
        if lower.contains(pat) {
            return "problem";
        }
    }

    // Milestone markers — shipped, completed, working
    let milestone_patterns = [
        "now working",
        "successfully",
        "shipped",
        "deployed",
        "it works",
        "tests pass",
        "all green",
        "breakthrough",
        "finally got",
        "got it working",
        "completed",
        "finished",
        "done with",
        "landed",
    ];
    for pat in &milestone_patterns {
        if lower.contains(pat) {
            return "milestone";
        }
    }

    // Preference markers — personal/operator preferences for style or workflow
    let preference_patterns = [
        "i prefer",
        "i like",
        "i don't like",
        "i want",
        "always use",
        "never use",
        "i usually",
        "my preference",
        "keep it",
        "avoid using",
    ];
    for pat in &preference_patterns {
        if lower.contains(pat) {
            return "preference";
        }
    }

    ""
}

impl Vein {
    const SESSION_REPORT_LIMIT: usize = 5;
    const SESSION_TURN_LIMIT: usize = 50;
    const IMPORT_FILE_LIMIT: usize = 12;
    const IMPORT_MAX_BYTES: u64 = 10 * 1024 * 1024;

    pub fn new<P: AsRef<Path>>(
        db_path: P,
        base_url: String,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Connection::open(db_path)?;

        // WAL mode for better concurrent read performance.
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // chunks_meta: tracks last-modified time per path for incremental indexing.
        // chunks_fts:  BM25 full-text index of all code chunks.
        // chunks_vec:  semantic embedding vectors, keyed by (path, chunk_idx).
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS chunks_meta (
                path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL,
                room TEXT NOT NULL DEFAULT 'root'
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                path UNINDEXED,
                content,
                tokenize='porter ascii'
            );
            CREATE TABLE IF NOT EXISTS chunks_vec (
                path TEXT NOT NULL,
                chunk_idx INTEGER NOT NULL,
                embedding BLOB NOT NULL,
                PRIMARY KEY (path, chunk_idx)
            );
            CREATE TABLE IF NOT EXISTS file_heat (
                path TEXT PRIMARY KEY,
                heat INTEGER NOT NULL DEFAULT 0,
                last_edit INTEGER NOT NULL DEFAULT 0
            );",
        )?;

        // Schema migrations — safe to run on every open (IF NOT EXISTS / ignored if col exists).
        let _ = db
            .execute_batch("ALTER TABLE chunks_meta ADD COLUMN room TEXT NOT NULL DEFAULT 'root';");
        let _ = db.execute_batch(
            "ALTER TABLE file_heat ADD COLUMN last_edit INTEGER NOT NULL DEFAULT 0;",
        );
        let _ = db.execute_batch(
            "ALTER TABLE chunks_meta ADD COLUMN memory_type TEXT NOT NULL DEFAULT '';",
        );

        Ok(Self {
            db: std::sync::Arc::new(std::sync::Mutex::new(db)),
            base_url,
        })
    }

    // ── Indexing ──────────────────────────────────────────────────────────────

    /// Index a single file for BM25 search. Skip if mtime hasn't changed.
    /// Returns the chunks that were written (empty if file was unchanged).
    pub fn index_document(
        &mut self,
        path: &str,
        last_modified: i64,
        full_text: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let room = detect_room(path);
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let chunks = chunk_by_symbols(ext, full_text);
        // Tag session memory with semantic type; source/doc chunks leave it empty.
        let memory_type = if room == "session" {
            detect_memory_type(full_text)
        } else {
            ""
        };
        self.index_chunks_with_room_and_type(path, last_modified, &room, memory_type, &chunks)
    }

    fn index_chunks_with_room_and_type(
        &mut self,
        path: &str,
        last_modified: i64,
        room: &str,
        memory_type: &str,
        chunks: &[String],
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let db = self.db.lock().unwrap();
        let existing: Option<i64> = db
            .query_row(
                "SELECT last_modified FROM chunks_meta WHERE path = ?1",
                params![path],
                |r| r.get(0),
            )
            .ok();

        if let Some(ts) = existing {
            if ts >= last_modified {
                return Ok(Vec::new()); // unchanged — skip
            }
        }

        // Evict stale BM25 chunks, stale embedding vectors, then update metadata.
        db.execute("DELETE FROM chunks_fts WHERE path = ?1", params![path])?;
        db.execute("DELETE FROM chunks_vec WHERE path = ?1", params![path])?;
        db.execute(
            "INSERT OR REPLACE INTO chunks_meta (path, last_modified, room, memory_type) VALUES (?1, ?2, ?3, ?4)",
            params![path, last_modified, room, memory_type],
        )?;

        drop(db);

        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        {
            let mut stmt = tx.prepare("INSERT INTO chunks_fts (path, content) VALUES (?1, ?2)")?;
            for chunk in chunks {
                stmt.execute(params![path, chunk.as_str()])?;
            }
        }
        tx.commit()?;

        Ok(chunks.to_vec())
    }

    /// Embed a set of chunks for one file and store the vectors.
    /// Called after `index_document` returns new chunks.
    /// Silently skips if the embedding model is unavailable.
    pub fn embed_and_store_chunks(&self, path: &str, chunks: &[String]) {
        for (idx, chunk) in chunks.iter().enumerate() {
            if let Some(vec) = embed_text_blocking(chunk, &self.base_url) {
                let blob = floats_to_blob(&vec);
                let db = self.db.lock().unwrap();
                let _ = db.execute(
                    "INSERT OR REPLACE INTO chunks_vec (path, chunk_idx, embedding) VALUES (?1, ?2, ?3)",
                    params![path, idx as i64, blob],
                );
            }
        }
    }

    // ── Search ────────────────────────────────────────────────────────────────

    /// BM25-ranked full-text search via FTS5 MATCH.
    pub fn search_bm25(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // Strip common English stopwords so FTS5 MATCH gets meaningful tokens only.
        // FTS5 uses implicit AND by default — passing stopwords like "how", "does",
        // "the" causes zero results because source code never contains those phrases.
        const STOPWORDS: &[&str] = &[
            "how", "does", "do", "did", "what", "where", "when", "why", "which", "who", "is",
            "are", "was", "were", "be", "been", "being", "have", "has", "had", "a", "an", "the",
            "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "from", "get",
            "gets", "got", "work", "works", "make", "makes", "use", "uses", "into", "that", "this",
            "it", "its",
        ];

        let safe_query: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' || c == '_' {
                    c
                } else {
                    ' '
                }
            })
            .collect();

        // Build an OR query from non-stopword tokens so any relevant term matches.
        let fts_query = safe_query
            .split_whitespace()
            .filter(|w| w.len() >= 3 && !STOPWORDS.contains(&w.to_lowercase().as_str()))
            .collect::<Vec<_>>()
            .join(" OR ");

        if fts_query.is_empty() {
            return Ok(Vec::new());
        }

        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT chunks_fts.path, chunks_fts.content, rank, cm.last_modified, cm.room, cm.memory_type
             FROM chunks_fts
             JOIN chunks_meta cm ON cm.path = chunks_fts.path
             WHERE chunks_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results: Vec<SearchResult> = stmt
            .query_map(params![fts_query, limit as i64], |row| {
                Ok(SearchResult {
                    path: row.get(0)?,
                    content: row.get(1)?,
                    score: -(row.get::<_, f64>(2).unwrap_or(0.0) as f32),
                    last_modified: row.get(3)?,
                    room: row.get(4)?,
                    memory_type: row.get::<_, String>(5).unwrap_or_default(),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Semantic search: embed the query, cosine-similarity against all stored vectors.
    /// Returns empty if the embedding model isn't loaded.
    pub fn search_semantic(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        let query_vec = match embed_query_blocking(query, &self.base_url) {
            Some(v) => v,
            None => return Vec::new(),
        };

        // Load all stored embeddings.
        let rows: Vec<(String, i64, Vec<u8>, i64, String, String)> = {
            let db = self.db.lock().unwrap();
            let mut stmt = match db.prepare(
                "SELECT cv.path, cv.chunk_idx, cv.embedding, cm.last_modified, cm.room, cm.memory_type
                 FROM chunks_vec cv
                 JOIN chunks_meta cm ON cm.path = cv.path",
            ) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5).unwrap_or_default(),
                ))
            })
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default()
        };

        if rows.is_empty() {
            return Vec::new();
        }

        // Score each chunk.
        let mut scored: Vec<(f32, String, i64, i64, String, String)> = rows
            .into_iter()
            .filter_map(|(path, idx, blob, last_modified, room, memory_type)| {
                let vec = blob_to_floats(&blob);
                let sim = cosine_similarity(&query_vec, &vec);
                Some((sim, path, idx, last_modified, room, memory_type))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Fetch the content for the top chunks.
        let db = self.db.lock().unwrap();
        scored
            .into_iter()
            .filter_map(|(score, path, idx, last_modified, room, memory_type)| {
                let content: Option<String> = db
                    .query_row(
                        "SELECT content FROM chunks_fts WHERE path = ?1 LIMIT 1 OFFSET ?2",
                        params![path, idx],
                        |r| r.get(0),
                    )
                    .ok();
                content.map(|c| SearchResult {
                    path,
                    content: c,
                    score,
                    room,
                    last_modified,
                    memory_type,
                })
            })
            .collect()
    }

    /// Hybrid search: BM25 + semantic, deduplicated and re-ranked.
    ///
    /// Semantic results are preferred (they score higher) when the embedding model
    /// is available. BM25 fills in or takes over when it isn't.
    /// Results from the active room (hottest subsystem by edit count) get a
    /// small boost so the model gravitates toward what's currently being worked on.
    pub fn search_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let candidate_limit = (limit.max(1) * 4).max(12);
        let bm25 = self.search_bm25(query, candidate_limit).unwrap_or_default();
        let semantic = self.search_semantic(query, candidate_limit);
        let signals = QuerySignals::from_query(query);

        // Determine the active room from heat scores.
        let active_room = self.active_room();

        // Merge: semantic results win ties (scored 1.0–2.0 range after boost).
        // BM25 results land in 0.0–1.0 range.
        let mut merged_by_path: HashMap<String, SearchResult> = HashMap::new();

        for r in semantic {
            let score = reranked_score(&signals, active_room.as_deref(), &r, true);
            merge_scored_result(&mut merged_by_path, SearchResult { score, ..r });
        }

        for r in bm25 {
            let score = reranked_score(&signals, active_room.as_deref(), &r, false);
            merge_scored_result(&mut merged_by_path, SearchResult { score, ..r });
        }

        let mut merged: Vec<SearchResult> = merged_by_path.into_values().collect();
        merged.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        merged.truncate(limit);
        Ok(merged)
    }

    /// Returns the room with the highest total heat (most edited subsystem).
    /// Used to bias retrieval toward what the user is actively working on.
    fn active_room(&self) -> Option<String> {
        let db = self.db.lock().unwrap();
        db.query_row(
            "SELECT cm.room, SUM(fh.heat) as total
             FROM file_heat fh
             JOIN chunks_meta cm ON cm.path = fh.path
             GROUP BY cm.room
             ORDER BY total DESC
             LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok()
    }

    // ── Project Indexing ──────────────────────────────────────────────────────

    /// Walk the entire project and index all source files (BM25 + embeddings).
    ///
    /// Skips: `target/`, `.git/`, `node_modules/`, `.hematite/`, files > 512 KB.
    /// Also indexes `.hematite/docs/` — the designated reference document drop folder.
    /// Returns the number of files processed (unchanged files are fast-pathed).
    pub fn index_project(&mut self) -> usize {
        let root = crate::tools::file_ops::workspace_root();
        let mut count = 0usize;

        const INDEXABLE: &[&str] = &[
            "rs", "toml", "md", "json", "ts", "tsx", "js", "py", "go", "c", "cpp", "h", "yaml",
            "yml", "txt",
        ];
        const SKIP_DIRS: &[&str] = &[
            "target",
            ".git",
            "node_modules",
            ".hematite",
        ];

        for entry in walkdir::WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    return !SKIP_DIRS.contains(&name.as_ref());
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !INDEXABLE.contains(&ext) {
                continue;
            }

            let Ok(meta) = std::fs::metadata(path) else {
                continue;
            };
            if meta.len() > 512_000 {
                continue;
            }

            let mtime = meta
                .modified()
                .map(|t| {
                    t.duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64
                })
                .unwrap_or(0);

            let rel = path.strip_prefix(&root).unwrap_or(path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");

            if let Ok(content) = std::fs::read_to_string(path) {
                match self.index_document(&rel_str, mtime, &content) {
                    Ok(new_chunks) if !new_chunks.is_empty() => {
                        count += 1;
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
        }

        count += self.index_workspace_artifacts(&crate::tools::file_ops::hematite_dir());

        count
    }

    /// Index workspace-local supporting context that should be available even
    /// outside a real project workspace: `.hematite/docs/`, recent session
    /// reports stored in `.hematite/reports/`, and imported chat exports in
    /// `.hematite/imports/`.
    pub fn index_workspace_artifacts(&mut self, workspace_root: &std::path::Path) -> usize {
        let mut count = self.index_docs_folder(workspace_root);
        count += self.index_recent_session_reports(workspace_root);
        count += self.index_imported_session_exports(workspace_root);
        self.backfill_missing_embeddings();
        count
    }

    /// Index reference documents in `.hematite/docs/`.
    /// Supports PDF (text extraction), markdown, and plain text.
    /// Documents are stored with path prefix `docs/filename` so they are
    /// distinguishable from source files in retrieval results.
    fn index_docs_folder(&mut self, workspace_root: &std::path::Path) -> usize {
        let docs_dir = workspace_root.join(".hematite").join("docs");
        const DOCS_INDEXABLE: &[&str] = &["pdf", "md", "txt", "markdown"];
        let mut count = 0usize;
        let mut desired_paths = HashSet::new();

        if docs_dir.exists() {
            for entry in walkdir::WalkDir::new(&docs_dir)
                .max_depth(3)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !DOCS_INDEXABLE.contains(&ext.as_str()) {
                    continue;
                }

                let Ok(meta) = std::fs::metadata(path) else {
                    continue;
                };
                if meta.len() > 50_000_000 {
                    continue;
                }

                let mtime = meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64
                    })
                    .unwrap_or(0);

                let rel = path.strip_prefix(workspace_root).unwrap_or(path);
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                desired_paths.insert(rel_str.clone());

                let content = if ext == "pdf" {
                    extract_pdf_text(path).ok().flatten()
                } else {
                    std::fs::read_to_string(path).ok()
                };

                if let Some(text) = content {
                    if text.trim().is_empty() {
                        continue;
                    }
                    match self.index_document(&rel_str, mtime, &text) {
                        Ok(new_chunks) if !new_chunks.is_empty() => {
                            count += 1;
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
        }

        self.prune_indexed_prefix(".hematite/docs/", &desired_paths);
        count
    }

    /// Index the most recent local session reports by exchange pair so prior
    /// decisions remain searchable across launches without flooding the vein.
    pub fn index_recent_session_reports(&mut self, workspace_root: &std::path::Path) -> usize {
        let reports_dir = workspace_root.join(".hematite").join("reports");
        let mut count = 0usize;
        let mut desired_paths = HashSet::new();

        if reports_dir.exists() {
            let mut reports: Vec<std::path::PathBuf> = std::fs::read_dir(&reports_dir)
                .ok()
                .into_iter()
                .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_file()
                        && path.extension().and_then(|ext| ext.to_str()) == Some("json")
                        && path
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .map(|stem| stem.starts_with("session_"))
                            .unwrap_or(false)
                })
                .collect();

            reports.sort_by(|a, b| {
                let a_name = a
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                let b_name = b
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                b_name.cmp(a_name)
            });
            reports.truncate(Self::SESSION_REPORT_LIMIT);

            for report_path in reports {
                let Ok(meta) = std::fs::metadata(&report_path) else {
                    continue;
                };
                let mtime = meta
                    .modified()
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64
                    })
                    .unwrap_or(0);

                for exchange in load_session_exchanges(&report_path, mtime) {
                    desired_paths.insert(exchange.path.clone());
                    let mtype = detect_memory_type(&exchange.content);
                    match self.index_chunks_with_room_and_type(
                        &exchange.path,
                        exchange.last_modified,
                        "session",
                        mtype,
                        std::slice::from_ref(&exchange.content),
                    ) {
                        Ok(new_chunks) if !new_chunks.is_empty() => {
                            count += 1;
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
        }

        self.prune_indexed_prefix("session/", &desired_paths);
        count
    }

    /// Index imported chat exports from `.hematite/imports/`.
    /// Supported inputs include already-normalized `>` transcripts, Claude Code
    /// JSONL, Codex CLI JSONL, simple role/content JSON exports, ChatGPT
    /// `mapping` exports, and Hematite session-report JSON.
    pub fn index_imported_session_exports(&mut self, workspace_root: &std::path::Path) -> usize {
        let imports_dir = workspace_root.join(".hematite").join("imports");
        let mut count = 0usize;
        let mut desired_paths = HashSet::new();

        if imports_dir.exists() {
            let mut imports: Vec<(std::path::PathBuf, i64)> = walkdir::WalkDir::new(&imports_dir)
                .max_depth(4)
                .follow_links(false)
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter(|entry| entry.file_type().is_file())
                .filter_map(|entry| {
                    let path = entry.into_path();
                    let ext = path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    if !matches!(ext.as_str(), "json" | "jsonl" | "md" | "txt") {
                        return None;
                    }
                    let meta = std::fs::metadata(&path).ok()?;
                    if meta.len() > Self::IMPORT_MAX_BYTES {
                        return None;
                    }
                    let mtime = meta
                        .modified()
                        .map(|t| {
                            t.duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64
                        })
                        .unwrap_or(0);
                    Some((path, mtime))
                })
                .collect();

            imports.sort_by(|(a_path, a_mtime), (b_path, b_mtime)| {
                b_mtime
                    .cmp(a_mtime)
                    .then_with(|| a_path.to_string_lossy().cmp(&b_path.to_string_lossy()))
            });
            imports.truncate(Self::IMPORT_FILE_LIMIT);

            for (import_path, mtime) in imports {
                for exchange in load_imported_session_exchanges(&import_path, &imports_dir, mtime) {
                    desired_paths.insert(exchange.path.clone());
                    let mtype = detect_memory_type(&exchange.content);
                    match self.index_chunks_with_room_and_type(
                        &exchange.path,
                        exchange.last_modified,
                        "session",
                        mtype,
                        std::slice::from_ref(&exchange.content),
                    ) {
                        Ok(new_chunks) if !new_chunks.is_empty() => {
                            count += 1;
                        }
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
            }
        }

        self.prune_indexed_prefix("session/imports/", &desired_paths);
        count
    }

    /// Embed any FTS chunks that don't yet have a vector in chunks_vec.
    /// Called at the end of index_project so that loading the embedding model
    /// after the initial index automatically triggers a semantic upgrade on the
    /// next agent turn — no /forget or file-touch required.
    fn backfill_missing_embeddings(&self) {
        // Fast path: if chunk counts match, nothing to do.
        let (fts_count, vec_count) = {
            let db = self.db.lock().unwrap();
            let fts: i64 = db
                .query_row("SELECT COUNT(*) FROM chunks_fts", [], |r| r.get(0))
                .unwrap_or(0);
            let vec: i64 = db
                .query_row("SELECT COUNT(*) FROM chunks_vec", [], |r| r.get(0))
                .unwrap_or(0);
            (fts, vec)
        };
        if fts_count == 0 || fts_count == vec_count {
            return;
        }

        // Fetch (path, chunk_idx, content) for chunks with no embedding.
        // chunks_fts rowid serves as chunk_idx (1-based → convert to 0-based).
        let missing: Vec<(String, i64, String)> = {
            let db = self.db.lock().unwrap();
            let mut stmt = db
                .prepare(
                    "SELECT f.path, (f.rowid - 1) AS chunk_idx, f.content
                     FROM chunks_fts f
                     LEFT JOIN chunks_vec v ON f.path = v.path AND (f.rowid - 1) = v.chunk_idx
                     WHERE v.path IS NULL
                     ORDER BY CASE
                         WHEN f.path LIKE '%.rs' THEN 0
                         WHEN f.path LIKE '%.toml' THEN 1
                         WHEN f.path LIKE '%.json' THEN 2
                         ELSE 3
                     END, f.path
                     LIMIT 20",
                )
                .unwrap();
            stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
        };

        for (path, idx, content) in missing {
            if let Some(vec) = embed_text_blocking(&content, &self.base_url) {
                let blob = floats_to_blob(&vec);
                let db = self.db.lock().unwrap();
                let _ = db.execute(
                    "INSERT OR REPLACE INTO chunks_vec (path, chunk_idx, embedding) VALUES (?1, ?2, ?3)",
                    params![path, idx, blob],
                );
            } else {
                // Embedding model not available — stop trying for this pass.
                break;
            }
        }
    }

    /// Total number of unique files currently indexed.
    /// Session exchange chunks are excluded so status counts stay source/doc centric.
    pub fn file_count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row(
            "SELECT COUNT(*) FROM chunks_meta WHERE path NOT LIKE 'session/%'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
    }

    /// Number of source/doc chunks that have semantic embedding vectors stored.
    /// Session exchange chunks are excluded so status counts stay source/doc centric.
    pub fn embedded_chunk_count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row(
            "SELECT COUNT(*) FROM chunks_vec WHERE path NOT LIKE 'session/%'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize
    }

    /// True when any chunk type currently has embeddings available.
    pub fn has_any_embeddings(&self) -> bool {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT EXISTS(SELECT 1 FROM chunks_vec LIMIT 1)", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0)
            != 0
    }

    /// Wipe all indexed data. The DB file stays on disk; next index_project()
    /// call rebuilds from scratch (re-reads all files, re-embeds all chunks).
    pub fn reset(&self) {
        let db = self.db.lock().unwrap();
        let _ = db.execute_batch(
            "DELETE FROM chunks_fts;
             DELETE FROM chunks_vec;
             DELETE FROM chunks_meta;",
        );
    }

    /// Return a compact operator-facing snapshot of what The Vein currently knows.
    /// Intended for trust/debug surfaces like `/vein-inspect`.
    pub fn inspect_snapshot(&self, hot_limit: usize) -> VeinInspectionSnapshot {
        let db = self.db.lock().unwrap();
        let indexed_source_files = db
            .query_row(
                "SELECT COUNT(*) FROM chunks_meta
                 WHERE path NOT LIKE 'session/%'
                   AND path NOT LIKE '.hematite/docs/%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let indexed_docs = db
            .query_row(
                "SELECT COUNT(*) FROM chunks_meta WHERE path LIKE '.hematite/docs/%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let indexed_session_exchanges = db
            .query_row(
                "SELECT COUNT(*) FROM chunks_meta WHERE path LIKE 'session/%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let embedded_source_doc_chunks = db
            .query_row(
                "SELECT COUNT(*) FROM chunks_vec WHERE path NOT LIKE 'session/%'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let has_any_embeddings = db
            .query_row("SELECT EXISTS(SELECT 1 FROM chunks_vec LIMIT 1)", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap_or(0)
            != 0;
        drop(db);

        let hot_files = self
            .hot_files(hot_limit.max(1))
            .into_iter()
            .map(|(path, heat, last_modified, room)| VeinHotFile {
                path,
                heat,
                last_modified,
                room,
            })
            .collect::<Vec<_>>();

        VeinInspectionSnapshot {
            indexed_source_files,
            indexed_docs,
            indexed_session_exchanges,
            embedded_source_doc_chunks,
            has_any_embeddings,
            active_room: self.active_room(),
            l1_ready: !hot_files.is_empty(),
            hot_files,
        }
    }

    // ── L1 heat tracking ──────────────────────────────────────────────────────

    /// Record an edit to a file. Increments its heat score in file_heat.
    /// Called from the tool dispatch after a successful edit_file / write_file /
    /// patch_hunk / multi_search_replace so the L1 context stays current.
    pub fn bump_heat(&self, path: &str) {
        if path.is_empty() {
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let db = self.db.lock().unwrap();
        let _ = db.execute(
            "INSERT INTO file_heat (path, heat, last_edit) VALUES (?1, 1, ?2)
             ON CONFLICT(path) DO UPDATE SET heat = heat + 1, last_edit = ?2",
            params![path, now],
        );
    }

    /// Return the top N hot files ranked by edit count (heat) then recency.
    /// Joins file_heat with chunks_meta so only indexed files are included.
    /// Returns (path, heat, mtime, room).
    fn hot_files(&self, n: usize) -> Vec<(String, i64, i64, String)> {
        let db = self.db.lock().unwrap();
        let mut stmt = match db.prepare(
            "SELECT fh.path, fh.heat, cm.last_modified, cm.room
             FROM file_heat fh
             JOIN chunks_meta cm ON cm.path = fh.path
             ORDER BY fh.heat DESC, cm.last_modified DESC
             LIMIT ?1",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map(params![n as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    }

    /// Return the paths of the top hot files (most edited).
    /// Used by RepoMapGenerator to bias PageRank toward active files.
    pub fn hot_file_paths(&self, n: usize) -> Vec<String> {
        self.hot_files(n)
            .into_iter()
            .map(|(path, _, _, _)| path)
            .collect()
    }

    /// Return hot files with normalized heat weights in [0.0, 1.0].
    /// The hottest file gets weight 1.0; others are scaled proportionally.
    /// Used by RepoMapGenerator to apply heat-weighted PageRank personalization.
    pub fn hot_files_weighted(&self, n: usize) -> Vec<(String, f64)> {
        let files = self.hot_files(n);
        if files.is_empty() {
            return vec![];
        }
        let max_heat = files
            .iter()
            .map(|(_, h, _, _)| *h)
            .max()
            .unwrap_or(1)
            .max(1) as f64;
        files
            .into_iter()
            .map(|(path, heat, _, _)| {
                let weight = (heat as f64) / max_heat;
                (path, weight)
            })
            .collect()
    }

    /// Build the L1 context block — a compact "hot files" summary injected into
    /// the system prompt at session start. Capped at ~150 tokens.
    /// Files are grouped by room so the model sees subsystem structure at a glance.
    /// Returns None when there are no heat records yet (fresh project).
    pub fn l1_context(&self) -> Option<String> {
        let files = self.hot_files(8);
        if files.is_empty() {
            return None;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Group by room for readability.
        let mut by_room: std::collections::BTreeMap<String, Vec<(String, i64, i64)>> =
            std::collections::BTreeMap::new();
        for (path, heat, mtime, room) in &files {
            by_room
                .entry(room.clone())
                .or_default()
                .push((path.clone(), *heat, *mtime));
        }

        let mut out = String::from("# Hot Files (most edited — grouped by subsystem)\n");
        for (room, entries) in &by_room {
            out.push_str(&format!("[{}]\n", room));
            for (path, heat, mtime) in entries {
                let age_secs = now - mtime;
                let age = if age_secs < 3600 {
                    "just now".to_string()
                } else if age_secs < 86400 {
                    format!("{}h ago", age_secs / 3600)
                } else {
                    format!("{}d ago", age_secs / 86400)
                };
                out.push_str(&format!(
                    "  - {} [{} edit{}, {}]\n",
                    path,
                    heat,
                    if *heat == 1 { "" } else { "s" },
                    age
                ));
            }
        }
        Some(out)
    }

    fn prune_indexed_prefix(&self, prefix: &str, desired_paths: &HashSet<String>) {
        let pattern = format!("{}%", prefix);
        let existing_paths: Vec<String> = {
            let db = self.db.lock().unwrap();
            let mut stmt = match db.prepare("SELECT path FROM chunks_meta WHERE path LIKE ?1") {
                Ok(stmt) => stmt,
                Err(_) => return,
            };
            stmt.query_map(params![pattern], |row| row.get::<_, String>(0))
                .map(|rows| rows.filter_map(|row| row.ok()).collect())
                .unwrap_or_default()
        };

        if existing_paths.is_empty() {
            return;
        }

        let db = self.db.lock().unwrap();
        for path in existing_paths {
            if desired_paths.contains(&path) {
                continue;
            }
            let _ = db.execute("DELETE FROM chunks_fts WHERE path = ?1", params![path]);
            let _ = db.execute("DELETE FROM chunks_vec WHERE path = ?1", params![path]);
            let _ = db.execute("DELETE FROM chunks_meta WHERE path = ?1", params![path]);
        }
    }
}

impl QuerySignals {
    fn from_query(query: &str) -> Self {
        let lower = query.to_ascii_lowercase();
        let historical_memory_hint = [
            "remember",
            "earlier",
            "previous",
            "last time",
            "what did we decide",
            "why did we decide",
            "what did we say",
            "why did we change",
        ]
        .iter()
        .any(|needle| lower.contains(needle));

        // Detect what memory type the query is asking about.
        let query_memory_type = if lower.contains("decide")
            || lower.contains("decision")
            || lower.contains("we agreed")
            || lower.contains("we chose")
        {
            Some("decision")
        } else if lower.contains("bug")
            || lower.contains("error")
            || lower.contains("issue")
            || lower.contains("problem")
            || lower.contains("fix")
            || lower.contains("broken")
        {
            Some("problem")
        } else if lower.contains("shipped")
            || lower.contains("milestone")
            || lower.contains("finished")
            || lower.contains("working now")
        {
            Some("milestone")
        } else if lower.contains("prefer")
            || lower.contains("my preference")
            || lower.contains("i like")
            || lower.contains("i want")
        {
            Some("preference")
        } else {
            None
        };

        Self {
            exact_phrases: extract_exact_phrases(query),
            standout_terms: extract_standout_terms(query),
            historical_memory_hint,
            temporal_reference: extract_temporal_reference(query),
            query_memory_type,
        }
    }
}

fn merge_scored_result(
    merged_by_path: &mut HashMap<String, SearchResult>,
    candidate: SearchResult,
) {
    match merged_by_path.get_mut(&candidate.path) {
        Some(existing) if candidate.score > existing.score => *existing = candidate,
        Some(_) => {}
        None => {
            merged_by_path.insert(candidate.path.clone(), candidate);
        }
    }
}

fn reranked_score(
    signals: &QuerySignals,
    active_room: Option<&str>,
    result: &SearchResult,
    is_semantic: bool,
) -> f32 {
    let base = if is_semantic {
        1.0 + result.score.clamp(0.0, 1.0)
    } else {
        (result.score / 10.0).clamp(0.0, 1.0)
    };
    base + room_bias(active_room, result)
        + retrieval_signal_boost(signals, result)
        + temporal_memory_boost(signals, result)
}

fn room_bias(active_room: Option<&str>, result: &SearchResult) -> f32 {
    if active_room == Some(result.room.as_str()) {
        0.15
    } else {
        0.0
    }
}

fn retrieval_signal_boost(signals: &QuerySignals, result: &SearchResult) -> f32 {
    let mut boost = 0.0f32;
    let haystack = format!(
        "{}\n{}",
        result.path.to_ascii_lowercase(),
        result.content.to_ascii_lowercase()
    );

    let phrase_matches = signals
        .exact_phrases
        .iter()
        .filter(|phrase| haystack.contains(phrase.as_str()))
        .count();
    if phrase_matches > 0 {
        boost += 0.35 + ((phrase_matches.saturating_sub(1)) as f32 * 0.1);
    }

    let mut standout_matches = 0;
    for term in &signals.standout_terms {
        if result.path.to_ascii_lowercase().contains(term.as_str()) {
            boost += 0.40;
            standout_matches += 1;
        } else if result.content.to_ascii_lowercase().contains(term.as_str()) {
            boost += 0.12;
            standout_matches += 1;
        }
        if standout_matches >= 3 {
            break;
        }
    }

    if signals.historical_memory_hint && result.room == "session" {
        boost += 0.45;
    }

    // Boost session chunks whose tagged memory type matches the query's intent.
    if let Some(qtype) = signals.query_memory_type {
        if !result.memory_type.is_empty() && result.memory_type == qtype {
            boost += 0.35;
        }
    }

    boost
}

fn temporal_memory_boost(signals: &QuerySignals, result: &SearchResult) -> f32 {
    if result.room != "session" {
        return 0.0;
    }
    let Some(reference) = signals.temporal_reference else {
        return 0.0;
    };
    let Some(memory_ts) = session_memory_timestamp(result) else {
        return 0.0;
    };

    let span = reference.window_secs.max(86_400);
    let full_fade = span.saturating_mul(8);
    if full_fade <= 0 {
        return 0.0;
    }

    let distance = (memory_ts - reference.target_ts).abs();
    let closeness = 1.0 - (distance as f32 / full_fade as f32).min(1.0);
    if closeness <= 0.0 {
        0.0
    } else {
        0.22 * closeness
    }
}

fn extract_exact_phrases(query: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let chars: Vec<char> = query.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let quote = chars[i];
        if !matches!(quote, '"' | '\'' | '`') {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < chars.len() && chars[end] != quote {
            end += 1;
        }
        if end > start {
            let phrase = chars[start..end]
                .iter()
                .collect::<String>()
                .trim()
                .to_ascii_lowercase();
            if phrase.len() >= 3 && !phrases.contains(&phrase) {
                phrases.push(phrase);
            }
        }
        i = end.saturating_add(1);
    }

    phrases
}

fn extract_standout_terms(query: &str) -> Vec<String> {
    const STOPWORDS: &[&str] = &[
        "about", "after", "before", "change", "changed", "decide", "decided", "does", "earlier",
        "flow", "from", "have", "into", "just", "last", "local", "make", "more", "remember",
        "should", "that", "their", "there", "these", "they", "this", "those", "what", "when",
        "where", "which", "why", "with", "work",
    ];

    let mut standout = Vec::new();
    for token in query.split(|ch: char| {
        !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
    }) {
        let trimmed = token.trim();
        if trimmed.len() < 4 {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) {
            continue;
        }

        let interesting = trimmed.chars().any(|ch| ch.is_ascii_digit())
            || trimmed
                .chars()
                .any(|ch| matches!(ch, '_' | '-' | '.' | '/' | ':'))
            || trimmed.chars().any(|ch| ch.is_ascii_uppercase())
            || trimmed.len() >= 9;

        if interesting && !standout.contains(&lower) {
            standout.push(lower);
        }
    }

    standout
}

fn extract_temporal_reference(query: &str) -> Option<TemporalReference> {
    if let Some(ts) = extract_iso_date_from_query(query) {
        return Some(TemporalReference {
            target_ts: ts,
            window_secs: 86_400,
        });
    }

    let now = current_unix_timestamp();
    let lower = query.to_ascii_lowercase();
    if lower.contains("yesterday") {
        Some(TemporalReference {
            target_ts: now.saturating_sub(86_400),
            window_secs: 86_400,
        })
    } else if lower.contains("today") || lower.contains("earlier today") {
        Some(TemporalReference {
            target_ts: now,
            window_secs: 86_400,
        })
    } else if lower.contains("last week") {
        Some(TemporalReference {
            target_ts: now.saturating_sub(7 * 86_400),
            window_secs: 7 * 86_400,
        })
    } else if lower.contains("last month") {
        Some(TemporalReference {
            target_ts: now.saturating_sub(30 * 86_400),
            window_secs: 30 * 86_400,
        })
    } else {
        None
    }
}

fn extract_iso_date_from_query(query: &str) -> Option<i64> {
    query
        .split(|ch: char| !(ch.is_ascii_digit() || ch == '-'))
        .find_map(parse_iso_date_token)
}

fn parse_iso_date_token(token: &str) -> Option<i64> {
    if token.len() != 10 {
        return None;
    }
    let bytes = token.as_bytes();
    if bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }

    let year = token.get(0..4)?.parse::<i32>().ok()?;
    let month = token.get(5..7)?.parse::<u32>().ok()?;
    let day = token.get(8..10)?.parse::<u32>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    Some(days_from_civil(year, month, day).saturating_mul(86_400))
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let yoe = year - era * 400;
    let month_prime = month as i32 + if month > 2 { -3 } else { 9 };
    let doy = (153 * month_prime + 2) / 5 + day as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146_097 + doe as i64 - 719_468
}

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn session_memory_timestamp(result: &SearchResult) -> Option<i64> {
    extract_session_path_timestamp(&result.path).or_else(|| {
        if result.last_modified > 0 {
            Some(result.last_modified)
        } else {
            None
        }
    })
}

fn extract_session_path_timestamp(path: &str) -> Option<i64> {
    let normalized = path.replace('\\', "/");
    let mut parts = normalized.split('/');
    if parts.next()? != "session" {
        return None;
    }
    parse_iso_date_token(parts.next()?)
}

fn session_speaker_kind(speaker: &str) -> SessionSpeakerKind {
    let normalized = speaker.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "you" | "user" => SessionSpeakerKind::User,
        "" | "system" | "tool" => SessionSpeakerKind::Ignore,
        _ => SessionSpeakerKind::Assistant,
    }
}

fn load_session_exchanges(report_path: &Path, last_modified: i64) -> Vec<SessionExchange> {
    let Ok(raw) = std::fs::read_to_string(report_path) else {
        return Vec::new();
    };
    let Ok(report) = serde_json::from_str::<SessionReport>(&raw) else {
        return Vec::new();
    };

    let session_key = report_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.strip_prefix("session_").or(Some(stem)))
        .unwrap_or("unknown-session")
        .to_string();
    let session_date = report
        .session_start
        .split('_')
        .next()
        .filter(|date| !date.is_empty())
        .unwrap_or_else(|| session_key.split('_').next().unwrap_or("unknown-date"))
        .to_string();

    let mut exchanges = Vec::new();
    let mut pending_user: Option<String> = None;
    let mut turn_index = 0usize;

    for entry in report.transcript {
        match session_speaker_kind(&entry.speaker) {
            SessionSpeakerKind::User => {
                let text = entry.text.trim();
                if !text.is_empty() {
                    pending_user = Some(text.to_string());
                }
            }
            SessionSpeakerKind::Assistant => {
                let text = entry.text.trim();
                if text.is_empty() {
                    continue;
                }
                let Some(user_text) = pending_user.take() else {
                    continue;
                };
                turn_index += 1;
                exchanges.push(SessionExchange {
                    path: format!(
                        "session/{}/{}/turn-{}",
                        session_date, session_key, turn_index
                    ),
                    last_modified,
                    content: format!(
                        "Earlier session exchange\nUser:\n{}\n\nAssistant:\n{}",
                        user_text, text
                    ),
                });
            }
            SessionSpeakerKind::Ignore => {}
        }
    }

    if exchanges.len() > Vein::SESSION_TURN_LIMIT {
        let keep_from = exchanges.len() - Vein::SESSION_TURN_LIMIT;
        exchanges = exchanges.into_iter().skip(keep_from).collect();
    }

    exchanges
}

fn load_imported_session_exchanges(
    import_path: &Path,
    imports_root: &Path,
    last_modified: i64,
) -> Vec<SessionExchange> {
    let Ok(raw) = std::fs::read_to_string(import_path) else {
        return Vec::new();
    };

    let messages = normalize_import_messages(&raw, import_path);
    if messages.is_empty() {
        return Vec::new();
    }

    let rel = import_path
        .strip_prefix(imports_root)
        .unwrap_or(import_path);
    let rel_slug = slugify_import_path(rel);
    let mut exchanges = Vec::new();
    let mut pending_user: Option<String> = None;
    let mut turn_index = 0usize;

    for (role, text) in messages {
        let cleaned = text.trim();
        if cleaned.is_empty() {
            continue;
        }
        match role.as_str() {
            "user" => pending_user = Some(cleaned.to_string()),
            "assistant" => {
                let Some(user_text) = pending_user.take() else {
                    continue;
                };
                turn_index += 1;
                exchanges.push(SessionExchange {
                    path: format!("session/imports/{}/turn-{}", rel_slug, turn_index),
                    last_modified,
                    content: format!(
                        "Imported session exchange\nSource: .hematite/imports/{}\n\nUser:\n{}\n\nAssistant:\n{}",
                        rel.to_string_lossy().replace('\\', "/"),
                        user_text,
                        cleaned
                    ),
                });
            }
            _ => {}
        }
    }

    if exchanges.len() > Vein::SESSION_TURN_LIMIT {
        let keep_from = exchanges.len() - Vein::SESSION_TURN_LIMIT;
        exchanges = exchanges.into_iter().skip(keep_from).collect();
    }

    exchanges
}

fn normalize_import_messages(raw: &str, import_path: &Path) -> Vec<(String, String)> {
    if raw.trim().is_empty() {
        return Vec::new();
    }

    if let Some(messages) = parse_marker_transcript(raw) {
        return messages;
    }

    let ext = import_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "json" | "jsonl")
        || matches!(raw.trim().chars().next(), Some('{') | Some('['))
    {
        if let Some(messages) = parse_jsonl_messages(raw) {
            if !messages.is_empty() {
                return messages;
            }
        }

        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            if let Some(messages) = parse_session_report_messages(&value) {
                return messages;
            }
            if let Some(messages) = parse_simple_role_messages(&value) {
                return messages;
            }
            if let Some(messages) = parse_chatgpt_mapping_messages(&value) {
                return messages;
            }
        }
    }

    Vec::new()
}

fn parse_marker_transcript(raw: &str) -> Option<Vec<(String, String)>> {
    let lines = raw.lines().collect::<Vec<_>>();
    if lines
        .iter()
        .filter(|line| line.trim_start().starts_with("> "))
        .count()
        < 2
    {
        return None;
    }

    let mut messages = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim_start();
        if let Some(rest) = line.strip_prefix("> ") {
            messages.push(("user".to_string(), rest.trim().to_string()));
            i += 1;
            let mut assistant_lines = Vec::new();
            while i < lines.len() {
                let next = lines[i];
                if next.trim_start().starts_with("> ") {
                    break;
                }
                let trimmed = next.trim();
                if !trimmed.is_empty() && trimmed != "---" {
                    assistant_lines.push(trimmed.to_string());
                }
                i += 1;
            }
            if !assistant_lines.is_empty() {
                messages.push(("assistant".to_string(), assistant_lines.join("\n")));
            }
        } else {
            i += 1;
        }
    }

    (!messages.is_empty()).then_some(messages)
}

fn parse_jsonl_messages(raw: &str) -> Option<Vec<(String, String)>> {
    let mut messages = Vec::new();
    let mut has_codex_session_meta = false;
    let mut saw_jsonl = false;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        saw_jsonl = true;
        let Some(object) = value.as_object() else {
            continue;
        };

        match object.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "session_meta" => {
                has_codex_session_meta = true;
            }
            "event_msg" => {
                let Some(payload) = object.get("payload").and_then(|v| v.as_object()) else {
                    continue;
                };
                let Some(text) = payload.get("message").and_then(|v| v.as_str()) else {
                    continue;
                };
                match payload.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                    "user_message" => messages.push(("user".to_string(), text.trim().to_string())),
                    "agent_message" => {
                        messages.push(("assistant".to_string(), text.trim().to_string()))
                    }
                    _ => {}
                }
            }
            "human" | "user" => {
                if let Some(text) = extract_text_content(object.get("message").unwrap_or(&value)) {
                    messages.push(("user".to_string(), text));
                }
            }
            "assistant" => {
                if let Some(text) = extract_text_content(object.get("message").unwrap_or(&value)) {
                    messages.push(("assistant".to_string(), text));
                }
            }
            _ => {
                if let Some(role) = object.get("role").and_then(|v| v.as_str()) {
                    if let Some(text) = extract_text_content(&value) {
                        match role {
                            "user" | "human" => messages.push(("user".to_string(), text)),
                            "assistant" | "ai" => messages.push(("assistant".to_string(), text)),
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    if !saw_jsonl {
        return None;
    }

    if has_codex_session_meta || !messages.is_empty() {
        return Some(messages);
    }

    None
}

fn parse_session_report_messages(value: &Value) -> Option<Vec<(String, String)>> {
    let report = value.as_object()?;
    let transcript = report.get("transcript")?.as_array()?;
    let mut messages = Vec::new();

    for entry in transcript {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let speaker = obj
            .get("speaker")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        if text.is_empty() {
            continue;
        }
        match session_speaker_kind(speaker) {
            SessionSpeakerKind::User => messages.push(("user".to_string(), text)),
            SessionSpeakerKind::Assistant => messages.push(("assistant".to_string(), text)),
            SessionSpeakerKind::Ignore => {}
        }
    }

    (!messages.is_empty()).then_some(messages)
}

fn parse_simple_role_messages(value: &Value) -> Option<Vec<(String, String)>> {
    if let Some(array) = value.as_array() {
        let messages = collect_role_messages(array);
        return (!messages.is_empty()).then_some(messages);
    }

    let obj = value.as_object()?;
    if let Some(messages_value) = obj.get("messages").or_else(|| obj.get("chat_messages")) {
        let array = messages_value.as_array()?;
        let messages = collect_role_messages(array);
        return (!messages.is_empty()).then_some(messages);
    }

    None
}

fn collect_role_messages(items: &[Value]) -> Vec<(String, String)> {
    let mut messages = Vec::new();
    for item in items {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(role) = obj.get("role").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(text) = extract_text_content(item) else {
            continue;
        };
        match role {
            "user" | "human" => messages.push(("user".to_string(), text)),
            "assistant" | "ai" => messages.push(("assistant".to_string(), text)),
            _ => {}
        }
    }
    messages
}

fn parse_chatgpt_mapping_messages(value: &Value) -> Option<Vec<(String, String)>> {
    let mapping = value.get("mapping")?.as_object()?;
    let mut current_id = mapping.iter().find_map(|(node_id, node)| {
        let obj = node.as_object()?;
        (obj.get("parent").is_some_and(|parent| parent.is_null())).then_some(node_id.clone())
    })?;

    let mut messages = Vec::new();
    let mut visited = std::collections::HashSet::new();

    while visited.insert(current_id.clone()) {
        let Some(node) = mapping.get(&current_id).and_then(|v| v.as_object()) else {
            break;
        };

        if let Some(message) = node.get("message") {
            let role = message
                .get("author")
                .and_then(|author| author.get("role"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(text) = extract_text_content(message) {
                match role {
                    "user" => messages.push(("user".to_string(), text)),
                    "assistant" => messages.push(("assistant".to_string(), text)),
                    _ => {}
                }
            }
        }

        let Some(next_id) = node
            .get("children")
            .and_then(|children| children.as_array())
            .and_then(|children| children.first())
            .and_then(|child| child.as_str())
        else {
            break;
        };
        current_id = next_id.to_string();
    }

    (!messages.is_empty()).then_some(messages)
}

fn extract_text_content(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then_some(trimmed.to_string());
    }

    if let Some(array) = value.as_array() {
        let joined = array
            .iter()
            .filter_map(extract_text_content)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        return (!joined.is_empty()).then_some(joined);
    }

    let obj = value.as_object()?;

    if let Some(content) = obj.get("content") {
        if let Some(text) = extract_text_content(content) {
            return Some(text);
        }
    }

    if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    if let Some(parts) = obj.get("parts").and_then(|v| v.as_array()) {
        let joined = parts
            .iter()
            .filter_map(|part| part.as_str().map(|s| s.trim().to_string()))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if !joined.is_empty() {
            return Some(joined);
        }
    }

    None
}

fn slugify_import_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '/' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('/')
        .replace('/', "__")
}

// ── Embedding API ─────────────────────────────────────────────────────────────

/// Call LM Studio's `/v1/embeddings` endpoint synchronously.
///
/// Uses nomic-embed-text-v2 MoE. Nomic v2 requires task instruction prefixes:
/// - Chunks stored in the index use `"search_document: "` prefix
/// - Queries at search time use `"search_query: "` prefix
/// LM Studio matches loaded models by substring so the quant suffix doesn't matter.
///
/// Returns `None` if:
/// - No embedding model is loaded in LM Studio
/// - LM Studio is not running
/// - Any network or parse error occurs
///
/// Callers must tolerate `None` and fall back to BM25-only search.
fn embed_text_blocking(text: &str, base_url: &str) -> Option<Vec<f32>> {
    embed_text_with_prefix(text, "search_document", base_url)
}

fn embed_query_blocking(text: &str, base_url: &str) -> Option<Vec<f32>> {
    embed_text_with_prefix(text, "search_query", base_url)
}

fn embed_text_with_prefix(text: &str, task: &str, base_url: &str) -> Option<Vec<f32>> {
    // Nomic v2 task instruction prefix format: "<task>: <text>"
    let prefixed = format!("{}: {}", task, text);
    // Truncate to ~8000 chars to stay within typical embedding model limits.
    let input = if prefixed.len() > 8000 {
        &prefixed[..8000]
    } else {
        &prefixed
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let body = serde_json::json!({
        "model": "nomic-embed-text-v2",
        "input": input
    });

    let url = format!("{}/v1/embeddings", base_url);
    let resp = client.post(&url).json(&body).send().ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().ok()?;
    let embedding = json["data"][0]["embedding"].as_array()?;
    let vec: Vec<f32> = embedding
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    if vec.is_empty() {
        None
    } else {
        Some(vec)
    }
}

// ── Vector math ───────────────────────────────────────────────────────────────

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

fn floats_to_blob(floats: &[f32]) -> Vec<u8> {
    floats.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn blob_to_floats(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect()
}

// ── Document extraction ───────────────────────────────────────────────────────

/// Extract plain text from a PDF file using pdf-extract.
/// Returns None if the file can't be read or yields no text.
/// Output is best-effort — layout is not preserved, but content is.
fn normalize_extracted_document_text(text: String) -> Option<String> {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = normalized.trim_matches(|c: char| c.is_whitespace() || c == '\0');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn extract_pdf_text_with_pdf_extract(path: &std::path::Path) -> Result<Option<String>, String> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text(path)
    }));
    std::panic::set_hook(previous_hook);

    match result {
        Ok(Ok(text)) => Ok(normalize_extracted_document_text(text)),
        Ok(Err(e)) => Err(format!("pdf-extract failed: {}", e)),
        Err(payload) => {
            let panic_text = if let Some(msg) = payload.downcast_ref::<&str>() {
                (*msg).to_string()
            } else if let Some(msg) = payload.downcast_ref::<String>() {
                msg.clone()
            } else {
                "unknown parser panic".to_string()
            };
            Err(format!("pdf-extract panicked: {}", panic_text))
        }
    }
}

fn extract_pdf_text_with_lopdf(path: &std::path::Path) -> Result<Option<String>, String> {
    let mut doc =
        lopdf::Document::load(path).map_err(|e| format!("lopdf could not open PDF: {}", e))?;

    if doc.is_encrypted() {
        doc.decrypt("")
            .map_err(|e| format!("PDF is encrypted and could not be decrypted: {}", e))?;
    }

    let page_numbers: Vec<u32> = doc.get_pages().keys().copied().collect();
    if page_numbers.is_empty() {
        return Ok(None);
    }

    let mut extracted_pages = Vec::new();
    let mut page_errors = Vec::new();

    for page_number in page_numbers {
        match doc.extract_text(&[page_number]) {
            Ok(text) => {
                if let Some(page_text) = normalize_extracted_document_text(text) {
                    extracted_pages.push(page_text);
                }
            }
            Err(e) => page_errors.push(format!("page {page_number}: {e}")),
        }
    }

    if !extracted_pages.is_empty() {
        return Ok(Some(extracted_pages.join("\n\n")));
    }

    if !page_errors.is_empty() {
        let sample_errors = page_errors
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!(
            "lopdf could not extract usable page text ({sample_errors})"
        ));
    }

    Ok(None)
}

fn extract_pdf_text_inside_helper(path: &std::path::Path) -> Result<Option<String>, String> {
    let mut failures = Vec::new();

    match extract_pdf_text_with_pdf_extract(path) {
        Ok(Some(text)) => return Ok(Some(text)),
        Ok(None) => failures.push("pdf-extract found no usable text".to_string()),
        Err(e) => failures.push(e),
    }

    match extract_pdf_text_with_lopdf(path) {
        Ok(Some(text)) => return Ok(Some(text)),
        Ok(None) => failures.push("lopdf found no usable text".to_string()),
        Err(e) => failures.push(e),
    }

    let detail = failures.into_iter().take(2).collect::<Vec<_>>().join("; ");
    Err(format!(
        "Could not extract text from PDF. Hematite keeps PDF parsing best-effort so it can stay a lightweight single-binary local coding harness. The file may be scanned/image-only, encrypted, or use unsupported font encoding. Try exporting it to text/markdown or attach page images instead. Detail: {}",
        detail
    ))
}

fn extract_pdf_text(path: &std::path::Path) -> Result<Option<String>, String> {
    let exe = std::env::current_exe()
        .map_err(|e| format!("Could not locate Hematite executable for PDF helper: {}", e))?;
    let output = std::process::Command::new(exe)
        .arg("--pdf-extract-helper")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("Could not launch PDF helper: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "PDF extraction failed.".to_string()
        } else {
            stderr
        });
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("PDF helper returned non-UTF8 text: {}", e))?;
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(text))
    }
}

pub fn run_pdf_extract_helper(path: &std::path::Path) -> i32 {
    match extract_pdf_text_inside_helper(path) {
        Ok(Some(text)) => {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            if stdout.write_all(text.as_bytes()).is_ok() {
                0
            } else {
                let _ = writeln!(
                    std::io::stderr(),
                    "PDF helper could not write extracted text."
                );
                1
            }
        }
        Ok(None) => {
            eprintln!(
                "Could not extract text from PDF. Hematite keeps PDF parsing best-effort so it can stay a lightweight single-binary local coding harness. The file appears to contain no usable embedded text. Try exporting it to text/markdown or attach page images instead."
            );
            1
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

/// Extract text from any supported document type (PDF, markdown, plain text).
/// Used by /attach for one-shot context injection.
pub fn extract_document_text(path: &std::path::Path) -> Result<String, String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "pdf" => {
            let text = extract_pdf_text(path)?.ok_or_else(|| {
                "PDF contains no extractable text — it may be scanned/image-only. \
                     Try attaching page screenshots with /image instead."
                    .to_string()
            })?;
            pdf_quality_check(text)
        }
        _ => std::fs::read_to_string(path).map_err(|e| format!("Could not read file: {e}")),
    }
}

/// Detect garbled PDF extraction — common with academic publisher PDFs that use
/// custom embedded fonts with non-standard glyph mappings.
///
/// Returns the text if it looks usable, or an informative error if it looks garbled.
fn pdf_quality_check(text: String) -> Result<String, String> {
    let trimmed = text.trim();

    // Too little content to be useful.
    if trimmed.len() < 150 {
        return Err(format!(
            "PDF extracted only {} characters — likely a scanned or image-only PDF, \
             or uses unsupported custom fonts. Try attaching page screenshots with /image instead.",
            trimmed.len()
        ));
    }

    // Detect words smashed together: space ratio too low.
    // Normal prose is ~15–20% spaces. Below 4% means glyphs aren't mapping to spaces.
    let non_newline: usize = trimmed.chars().filter(|c| *c != '\n' && *c != '\r').count();
    let spaces: usize = trimmed.chars().filter(|c| *c == ' ').count();
    let space_ratio = if non_newline > 0 {
        spaces as f32 / non_newline as f32
    } else {
        0.0
    };

    if space_ratio < 0.04 {
        return Err(
            "PDF text extraction produced garbled output — words are merged with no spaces. \
             This usually means the PDF uses custom embedded fonts (common with academic publishers \
             like EBSCO, Elsevier, Springer). \
             Try a PDF exported from Word, Google Docs, or LaTeX, \
             or attach page screenshots with /image instead.".to_string()
        );
    }

    Ok(text)
}

// ── Chunking strategies ───────────────────────────────────────────────────────

/// Dispatch to the correct chunking strategy based on file extension.
fn chunk_by_symbols(ext: &str, text: &str) -> Vec<String> {
    if ext == "rs" {
        chunk_rust_symbols(text)
    } else {
        chunk_paragraphs(text)
    }
}

/// Chunk Rust source at top-level item boundaries.
///
/// Detects lines at column 0 that start a Rust declaration keyword, flushes
/// the accumulated buffer, then moves any trailing doc-comments / attributes
/// forward so they stay with the item they annotate.
///
/// Items larger than 3000 chars (e.g. large impl blocks) are further split
/// by sliding window so no single chunk blows the retrieval budget.
fn chunk_rust_symbols(text: &str) -> Vec<String> {
    const ITEM_STARTS: &[&str] = &[
        "pub fn ",
        "pub async fn ",
        "pub unsafe fn ",
        "async fn ",
        "unsafe fn ",
        "fn ",
        "pub impl",
        "impl ",
        "pub struct ",
        "struct ",
        "pub enum ",
        "enum ",
        "pub trait ",
        "trait ",
        "pub mod ",
        "mod ",
        "pub type ",
        "type ",
        "pub const ",
        "const ",
        "pub static ",
        "static ",
    ];

    let lines: Vec<&str> = text.lines().collect();
    let mut chunks: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();

    for &line in &lines {
        let top_level = !line.starts_with(' ') && !line.starts_with('\t');
        let is_item = top_level && ITEM_STARTS.iter().any(|s| line.starts_with(s));

        if is_item && !current.is_empty() {
            // Scan backward to find where trailing doc-comments / attributes start —
            // move them to the new chunk so they land with their item.
            let mut split = current.len();
            while split > 0 {
                let prev = current[split - 1].trim();
                if prev.starts_with("///")
                    || prev.starts_with("//!")
                    || prev.starts_with("#[")
                    || prev.is_empty()
                {
                    split -= 1;
                } else {
                    break;
                }
            }
            let body = current[..split].join("\n");
            if !body.trim().is_empty() {
                chunks.push(body);
            }
            current = current[split..].to_vec();
        }
        current.push(line);
    }
    if !current.is_empty() {
        let body = current.join("\n");
        if !body.trim().is_empty() {
            chunks.push(body);
        }
    }

    // Subdivide any oversized blocks (e.g. long impl blocks with many methods).
    let mut result = Vec::new();
    for chunk in chunks {
        if chunk.len() > 3000 {
            result.extend(sliding_window_chunks(&chunk, 2000, 200));
        } else {
            result.push(chunk);
        }
    }
    result
}

/// Chunk non-Rust text at paragraph boundaries (double newline).
fn chunk_paragraphs(text: &str) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();

    for para in text.split("\n\n") {
        if current.len() + para.len() + 2 > 2000 {
            if !current.trim().is_empty() {
                result.push(current.clone());
            }
            current = para.to_string();
        } else {
            if !current.is_empty() {
                current.push_str("\n\n");
            }
            current.push_str(para);
        }
    }
    if !current.trim().is_empty() {
        result.push(current);
    }

    let mut final_result = Vec::new();
    for chunk in result {
        if chunk.len() > 2000 {
            final_result.extend(sliding_window_chunks(&chunk, 2000, 200));
        } else {
            final_result.push(chunk);
        }
    }
    final_result
}

/// Classic sliding-window fallback for oversized blocks.
fn sliding_window_chunks(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + chunk_size).min(chars.len());
        result.push(chars[i..end].iter().collect());
        if end == chars.len() {
            break;
        }
        i += chunk_size - overlap;
    }
    result
}
