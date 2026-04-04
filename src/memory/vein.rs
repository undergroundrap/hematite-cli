use rusqlite::{params, Connection};
use std::path::Path;

/// "The Vein" — local RAG memory engine backed by SQLite FTS5.
///
/// Indexes all source files in the project using sliding-window chunking.
/// Searches via BM25 full-text ranking — no external embedding model required.
/// Incremental: only re-indexes files whose mtime has changed.
pub struct Vein {
    db: std::sync::Arc<std::sync::Mutex<Connection>>,
}

// SAFETY: rusqlite::Connection is !Send by default, but we wrap it in Arc<Mutex>
// and ensure all accesses are synchronized. We primarily use this for FTS5
// indexing which is safe across threads when serialized by a mutex.
unsafe impl Send for Vein {}
unsafe impl Sync for Vein {}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub content: String,
    /// BM25 relevance score (higher = more relevant).
    pub score: f32,
}

impl Vein {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let db = Connection::open(db_path)?;

        // WAL mode for better concurrent read performance.
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        // Metadata table: tracks last-modified time per path.
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS chunks_meta (
                path TEXT PRIMARY KEY,
                last_modified INTEGER NOT NULL
            );
            CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
                path UNINDEXED,
                content,
                tokenize='porter ascii'
            );",
        )?;

        Ok(Self { db: std::sync::Arc::new(std::sync::Mutex::new(db)) })
    }

    /// Index a single file. Skip if mtime hasn't changed since last index.
    pub fn index_document(
        &mut self,
        path: &str,
        last_modified: i64,
        full_text: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                return Ok(());
            }
        }

        // Evict stale chunks then update metadata.
        db.execute("DELETE FROM chunks_fts WHERE path = ?1", params![path])?;
        db.execute(
            "INSERT OR REPLACE INTO chunks_meta (path, last_modified) VALUES (?1, ?2)",
            params![path, last_modified],
        )?;

        // Symbol-aware chunker: groups complete Rust items (fn/impl/struct/enum/trait)
        // into single chunks so the model always retrieves coherent code units.
        // Non-Rust files chunk at paragraph boundaries. Falls back to sliding window
        // for oversized blocks.
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let chunks = chunk_by_symbols(ext, full_text);

        // Close the local lock so we can start a transaction on the connection.
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

        Ok(())
    }

    /// BM25-ranked full-text search via FTS5 MATCH.
    /// Returns the top `limit` chunks ordered by relevance.
    pub fn search_context(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
        // Sanitize: FTS5 MATCH is sensitive to unbalanced quotes and special tokens.
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
        let safe_query = safe_query.trim().to_string();
        if safe_query.is_empty() {
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
            .query_map(params![safe_query, limit as i64], |row| {
                Ok(SearchResult {
                    path: row.get(0)?,
                    content: row.get(1)?,
                    // FTS5 rank is negative (lower = better). Negate for "higher = better".
                    score: -(row.get::<_, f64>(2).unwrap_or(0.0) as f32),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Walk the entire project and index all source files.
    ///
    /// Skips: `target/`, `.git/`, `node_modules/`, `.hematite/`, files > 100 KB.
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
            if meta.len() > 100_000 {
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
                if self.index_document(&rel_str, mtime, &content).is_ok() {
                    count += 1;
                }
            }
        }

        count
    }

    /// Total number of unique files currently indexed.
    pub fn file_count(&self) -> usize {
        let db = self.db.lock().unwrap();
        db.query_row("SELECT COUNT(*) FROM chunks_meta", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap_or(0) as usize
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
/// Accumulates paragraphs up to 2000 chars; oversized paragraphs fall back to
/// sliding window.
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
