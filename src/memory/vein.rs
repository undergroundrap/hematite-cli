use rusqlite::{params, Connection};
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
}

impl Vein {
    pub fn new<P: AsRef<Path>>(db_path: P, base_url: String) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Connection::open(db_path)?;

        // WAL mode for better concurrent read performance.
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // chunks_meta: tracks last-modified time per path for incremental indexing.
        // chunks_fts:  BM25 full-text index of all code chunks.
        // chunks_vec:  semantic embedding vectors, keyed by (path, chunk_idx).
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS chunks_meta (
                path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL
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
            );",
        )?;

        Ok(Self { db: std::sync::Arc::new(std::sync::Mutex::new(db)), base_url })
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
            "INSERT OR REPLACE INTO chunks_meta (path, last_modified) VALUES (?1, ?2)",
            params![path, last_modified],
        )?;

        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let chunks = chunk_by_symbols(ext, full_text);

        drop(db);

        let mut db = self.db.lock().unwrap();
        let tx = db.transaction()?;
        {
            let mut stmt =
                tx.prepare("INSERT INTO chunks_fts (path, content) VALUES (?1, ?2)")?;
            for chunk in &chunks {
                stmt.execute(params![path, chunk.as_str()])?;
            }
        }
        tx.commit()?;

        Ok(chunks)
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
            "how", "does", "do", "did", "what", "where", "when", "why", "which", "who",
            "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
            "a", "an", "the", "and", "or", "but", "in", "on", "at", "to", "for",
            "of", "with", "by", "from", "get", "gets", "got", "work", "works",
            "make", "makes", "use", "uses", "into", "that", "this", "it", "its",
        ];

        let safe_query: String = query
            .chars()
            .map(|c| if c.is_alphanumeric() || c == ' ' || c == '_' { c } else { ' ' })
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
            "SELECT path, content, rank
             FROM chunks_fts
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
        let rows: Vec<(String, i64, Vec<u8>)> = {
            let db = self.db.lock().unwrap();
            let mut stmt = match db.prepare(
                "SELECT cv.path, cv.chunk_idx, cv.embedding
                 FROM chunks_vec cv",
            ) {
                Ok(s) => s,
                Err(_) => return Vec::new(),
            };
            stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, Vec<u8>>(2)?,
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
        let mut scored: Vec<(f32, String, i64)> = rows
            .into_iter()
            .filter_map(|(path, idx, blob)| {
                let vec = blob_to_floats(&blob);
                let sim = cosine_similarity(&query_vec, &vec);
                Some((sim, path, idx))
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        // Fetch the content for the top chunks.
        let db = self.db.lock().unwrap();
        scored
            .into_iter()
            .filter_map(|(score, path, idx)| {
                // chunks_fts rows for this path are indexed in insertion order = chunk order.
                // We use LIMIT/OFFSET to fetch the chunk at position `idx`.
                let content: Option<String> = db
                    .query_row(
                        "SELECT content FROM chunks_fts WHERE path = ?1 LIMIT 1 OFFSET ?2",
                        params![path, idx],
                        |r| r.get(0),
                    )
                    .ok();
                content.map(|c| SearchResult { path, content: c, score })
            })
            .collect()
    }

    /// Hybrid search: BM25 + semantic, deduplicated and re-ranked.
    ///
    /// Semantic results are preferred (they score higher) when the embedding model
    /// is available. BM25 fills in or takes over when it isn't.
    pub fn search_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        let bm25 = self.search_bm25(query, limit).unwrap_or_default();
        let semantic = self.search_semantic(query, limit);

        // Merge: semantic results win ties (scored 1.0–2.0 range after boost).
        // BM25 results land in 0.0–1.0 range.
        let mut merged: Vec<SearchResult> = Vec::new();

        for r in semantic {
            // Boost semantic scores into the 1.0–2.0 band.
            merged.push(SearchResult { score: 1.0 + r.score.clamp(0.0, 1.0), ..r });
        }

        for r in bm25 {
            // Only add BM25 results that aren't already covered by a semantic hit.
            if !merged.iter().any(|m| m.path == r.path) {
                // Normalize BM25 score into 0.0–1.0: raw BM25 scores vary, cap at 10.
                let norm = (r.score / 10.0).clamp(0.0, 1.0);
                merged.push(SearchResult { score: norm, ..r });
            }
        }

        merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        merged.truncate(limit);
        Ok(merged)
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
            "rs", "toml", "md", "json", "ts", "tsx", "js", "py", "go",
            "c", "cpp", "h", "yaml", "yml", "txt",
        ];
        const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules", ".hematite"];

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

            let Ok(meta) = std::fs::metadata(path) else { continue };
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

        // Index reference documents dropped into .hematite/docs/
        // Supports PDFs plus plain text/markdown — unified with source code retrieval.
        count += self.index_docs_folder(&root);

        // Embed any unembedded chunks (new files + files whose mtime changed).
        // Capped at 20 per call so startup completes quickly; remaining chunks
        // are filled on subsequent turns.
        self.backfill_missing_embeddings();

        count
    }

    /// Index reference documents in `.hematite/docs/`.
    /// Supports PDF (text extraction), markdown, and plain text.
    /// Documents are stored with path prefix `docs/filename` so they are
    /// distinguishable from source files in retrieval results.
    fn index_docs_folder(&mut self, workspace_root: &std::path::Path) -> usize {
        let docs_dir = workspace_root.join(".hematite").join("docs");
        if !docs_dir.exists() {
            return 0;
        }

        const DOCS_INDEXABLE: &[&str] = &["pdf", "md", "txt", "markdown"];
        let mut count = 0usize;

        for entry in walkdir::WalkDir::new(&docs_dir)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !DOCS_INDEXABLE.contains(&ext.as_str()) {
                continue;
            }

            let Ok(meta) = std::fs::metadata(path) else { continue };
            if meta.len() > 50_000_000 { // 50 MB cap for docs
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
                Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?))
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
    pub fn file_count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT COUNT(*) FROM chunks_meta", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// Number of chunks that have semantic embedding vectors stored.
    pub fn embedded_chunk_count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT COUNT(*) FROM chunks_vec", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
    }

    /// Wipe all indexed data. The DB file stays on disk; next index_project()
    /// call rebuilds from scratch (re-reads all files, re-embeds all chunks).
    pub fn reset(&self) {
        let db = self.db.lock().unwrap();
        let _ = db.execute_batch(
            "DELETE FROM chunks_fts;
             DELETE FROM chunks_vec;
             DELETE FROM chunks_meta;"
        );
    }
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
    let input = if prefixed.len() > 8000 { &prefixed[..8000] } else { &prefixed };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    let body = serde_json::json!({
        "model": "nomic-embed-text-v2",
        "input": input
    });

    let url = format!("{}/v1/embeddings", base_url);
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().ok()?;
    let embedding = json["data"][0]["embedding"].as_array()?;
    let vec: Vec<f32> = embedding
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    if vec.is_empty() { None } else { Some(vec) }
}

// ── Vector math ───────────────────────────────────────────────────────────────

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { 0.0 } else { dot / (norm_a * norm_b) }
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
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text(path)
    }));
    std::panic::set_hook(previous_hook);

    match result {
        Ok(Ok(text)) => {
            use std::io::Write;
            let mut stdout = std::io::stdout();
            if stdout.write_all(text.as_bytes()).is_ok() {
                0
            } else {
                let _ = writeln!(std::io::stderr(), "PDF helper could not write extracted text.");
                1
            }
        }
        Ok(Err(e)) => {
            eprintln!("PDF extraction failed: {}", e);
            1
        }
        Err(payload) => {
            let panic_text = if let Some(msg) = payload.downcast_ref::<&str>() {
                (*msg).to_string()
            } else if let Some(msg) = payload.downcast_ref::<String>() {
                msg.clone()
            } else {
                "unknown pdf parser panic".to_string()
            };
            eprintln!("PDF extraction panicked: {}", panic_text);
            1
        }
    }
}

/// Extract text from any supported document type (PDF, markdown, plain text).
/// Used by /attach for one-shot context injection.
pub fn extract_document_text(path: &std::path::Path) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => extract_pdf_text(path)?
            .ok_or_else(|| "Could not extract text from PDF — file may be scanned/image-only or use unsupported font encoding.".to_string()),
        _ => std::fs::read_to_string(path)
            .map_err(|e| format!("Could not read file: {e}")),
    }
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
        "pub fn ", "pub async fn ", "pub unsafe fn ",
        "async fn ", "unsafe fn ", "fn ",
        "pub impl", "impl ",
        "pub struct ", "struct ",
        "pub enum ", "enum ",
        "pub trait ", "trait ",
        "pub mod ", "mod ",
        "pub type ", "type ",
        "pub const ", "const ",
        "pub static ", "static ",
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
                if prev.starts_with("///") || prev.starts_with("//!")
                    || prev.starts_with("#[") || prev.is_empty()
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
