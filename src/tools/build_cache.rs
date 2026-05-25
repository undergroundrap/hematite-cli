use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;
use walkdir::WalkDir;

struct CacheEntry {
    ok: bool,
    output: String,
    hits: u32,
}

static CACHE: OnceLock<Mutex<HashMap<String, CacheEntry>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, CacheEntry>> {
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn is_artifact(path: &Path) -> bool {
    path.components().any(|c| {
        matches!(
            c.as_os_str().to_string_lossy().as_ref(),
            "target" | "node_modules" | ".git" | "dist" | "build" | "__pycache__" | ".hematite"
        )
    })
}

/// SHA-256 of (command + sorted source file mtimes). Fast: only reads metadata, not content.
pub fn compute_key(cwd: &Path, command: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(command.as_bytes());
    hasher.update(b"\x00");

    let mut entries: Vec<(String, u64)> = WalkDir::new(cwd)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && !is_artifact(e.path()))
        .filter_map(|e| {
            let rel = e
                .path()
                .strip_prefix(cwd)
                .ok()?
                .to_string_lossy()
                .into_owned();
            let mtime = e
                .metadata()
                .ok()?
                .modified()
                .ok()?
                .duration_since(SystemTime::UNIX_EPOCH)
                .ok()?
                .as_secs();
            Some((rel, mtime))
        })
        .collect();

    entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    for (path, mtime) in &entries {
        hasher.update(path.as_bytes());
        hasher.update(b":");
        hasher.update(mtime.to_le_bytes());
        hasher.update(b"\n");
    }

    format!("{:x}", hasher.finalize())
}

/// Returns `Some((ok, output, hit_count))` on cache hit.
pub fn get(key: &str) -> Option<(bool, String, u32)> {
    let cache = cache().lock().ok()?;
    let entry = cache.get(key)?;
    Some((entry.ok, entry.output.clone(), entry.hits))
}

/// Records a build result. Only caches successful builds and passing test runs.
pub fn insert(key: String, ok: bool, output: String) {
    if !ok {
        return;
    }
    if let Ok(mut cache) = cache().lock() {
        cache.insert(
            key,
            CacheEntry {
                ok,
                output,
                hits: 0,
            },
        );
    }
}

/// Increments the hit counter for a key and returns the new count.
pub fn record_hit(key: &str) -> u32 {
    if let Ok(mut cache) = cache().lock() {
        if let Some(entry) = cache.get_mut(key) {
            entry.hits += 1;
            return entry.hits;
        }
    }
    0
}

/// Invalidates all cached entries — called after any file edit.
pub fn invalidate_all() {
    if let Ok(mut cache) = cache().lock() {
        cache.clear();
    }
}

/// Returns (total_entries, total_hits) for status reporting.
pub fn stats() -> (usize, u32) {
    if let Ok(cache) = cache().lock() {
        let entries = cache.len();
        let hits: u32 = cache.values().map(|e| e.hits).sum();
        (entries, hits)
    } else {
        (0, 0)
    }
}
