// Embedding-based intent classifier — semantic pre-filter for routing decisions.
//
// Uses nomic-embed-text-v2 (already loaded in LM Studio alongside the main model)
// to verify whether a user query is genuinely diagnostic or conversational.
//
// When the keyword router would inject HOST INSPECTION MODE, this classifier runs
// as a second-opinion pass. If it returns Advisory with high confidence, the
// injection is suppressed and the model answers from context instead of fetching
// fresh machine data.
//
// Centroids are bootstrapped lazily on first use by batch-embedding a small set
// of labeled example phrases (~100ms total, one API call). Subsequent calls embed
// only the query (~50ms). Falls back to Ambiguous silently if the embed model is
// unavailable or slow — keyword routing continues as before.

use tokio::sync::OnceCell;

// ── Public API ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntentClass {
    /// Conversational, advisory, or declarative. Suppress HOST INSPECTION MODE —
    /// the model should answer from existing context, not fetch new data.
    Advisory,
    /// Clearly diagnostic. The keyword router's topic choice is correct.
    Diagnostic,
    /// Uncertain. Defer to keyword router as before.
    Ambiguous,
}

/// Classify user query intent using embedding similarity against labeled centroids.
///
/// Only called when the keyword router has already returned `host_inspection_mode = true`,
/// so this is a veto path, not the primary routing path. Returning Ambiguous is always
/// safe — it just falls through to existing behavior.
pub async fn classify_intent(query: &str, api_url: &str) -> IntentClass {
    let centroids = ensure_centroids(api_url).await;
    let (adv_centroid, diag_centroid) = match centroids {
        Some(c) => c,
        None => return IntentClass::Ambiguous,
    };

    let query_vec = match embed_query(query, api_url).await {
        Some(v) => v,
        None => return IntentClass::Ambiguous,
    };

    let advisory_score = cosine_similarity(&query_vec, adv_centroid);
    let diagnostic_score = cosine_similarity(&query_vec, diag_centroid);

    classify_from_scores(advisory_score, diagnostic_score)
}

// ── Centroid bootstrap ────────────────────────────────────────────────────────

// Stored as (advisory_centroid, diagnostic_centroid). None = embed model unavailable.
static CENTROIDS: OnceCell<Option<(Vec<f32>, Vec<f32>)>> = OnceCell::const_new();

async fn ensure_centroids(api_url: &str) -> Option<&'static (Vec<f32>, Vec<f32>)> {
    let url = api_url.to_string();
    let opt = CENTROIDS
        .get_or_init(|| async move { compute_centroids(&url).await })
        .await;
    opt.as_ref()
}

async fn compute_centroids(api_url: &str) -> Option<(Vec<f32>, Vec<f32>)> {
    let adv_vecs = embed_batch(ADVISORY_EXAMPLES, api_url).await?;
    let diag_vecs = embed_batch(DIAGNOSTIC_EXAMPLES, api_url).await?;

    let adv_centroid = mean_centroid(&adv_vecs)?;
    let diag_centroid = mean_centroid(&diag_vecs)?;

    eprintln!(
        "[intent_embed] centroids ready ({} advisory, {} diagnostic examples)",
        adv_vecs.len(),
        diag_vecs.len()
    );
    Some((adv_centroid, diag_centroid))
}

// ── Example phrases ───────────────────────────────────────────────────────────

// Advisory examples — the model should NOT call inspect_host for these.
// Covers: opinion questions, hypotheticals, declarative statements, acknowledgments.
const ADVISORY_EXAMPLES: &[&str] = &[
    "would more ram help with this",
    "should I upgrade my GPU",
    "is that worth buying",
    "could I offload VRAM to system RAM",
    "i think the cpu is fine",
    "what if I had a faster SSD",
    "makes sense so the network is slow",
    "so the ram is the issue right",
    "do you think I should restart",
    "is it worth getting more storage",
    "if i upgraded the gpu would that help",
    "i believe the service is running",
    "i see the memory is fine",
    "everything looks good here",
    "ok so the cpu is at 8 percent that seems fine",
    "i think the service is already running",
    "my vram situation seems to be improving",
    "makes sense that the disk would be slow",
    "yeah that all adds up",
    "so the network was just congested",
    "that explains why the gpu was hot",
    "ah ok so it was the ram all along",
    "i guess the service crashed overnight",
    "would adding another monitor hurt gpu performance",
    "so basically the ssd is the bottleneck right",
];

// Diagnostic examples — the model SHOULD call inspect_host for these.
// Covers: data requests, status checks, show/list/check commands.
const DIAGNOSTIC_EXAMPLES: &[&str] = &[
    "how much RAM do I have",
    "show me running processes",
    "what is my CPU usage right now",
    "check my disk health",
    "why is my PC slow",
    "what services are running",
    "list my network adapters",
    "what GPU do I have",
    "is my firewall on",
    "show me recent errors",
    "what is my IP address",
    "check my wifi signal strength",
    "how much free disk space do I have",
    "what is taking up all my memory",
    "show hardware specs",
    "what processes are using the most RAM",
    "is my bluetooth working",
    "check my disk for errors",
    "what network connections are active",
    "show me the system logs",
    "what is the cpu temperature",
    "are there any pending windows updates",
    "is the docker daemon running",
    "check my battery status",
    "what is my gpu driver version",
];

// ── Embedding helpers ─────────────────────────────────────────────────────────

async fn embed_query(text: &str, api_url: &str) -> Option<Vec<f32>> {
    // nomic-embed-text-v2 uses task instruction prefixes
    let input = format!("search_query: {text}");
    embed_single(&input, api_url).await
}

async fn embed_batch(texts: &[&str], api_url: &str) -> Option<Vec<Vec<f32>>> {
    // Batch embed with document prefix — one API call for all examples
    let inputs: Vec<String> = texts
        .iter()
        .map(|t| format!("search_document: {t}"))
        .collect();

    let body = serde_json::json!({
        "model": "nomic-embed-text-v2",
        "input": inputs
    });

    let url = format!("{}/v1/embeddings", api_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .ok()?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let data = json["data"].as_array()?;

    let vecs: Vec<Vec<f32>> = data
        .iter()
        .filter_map(|item| {
            item["embedding"].as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64().map(|f| f as f32))
                    .collect()
            })
        })
        .collect();

    if vecs.len() != texts.len() {
        None
    } else {
        Some(vecs)
    }
}

async fn embed_single(input: &str, api_url: &str) -> Option<Vec<f32>> {
    let body = serde_json::json!({
        "model": "nomic-embed-text-v2",
        "input": input
    });

    let url = format!("{}/v1/embeddings", api_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .ok()?;

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let arr = json["data"][0]["embedding"].as_array()?;
    let vec: Vec<f32> = arr
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
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn mean_centroid(vecs: &[Vec<f32>]) -> Option<Vec<f32>> {
    if vecs.is_empty() {
        return None;
    }
    let dim = vecs[0].len();
    if dim == 0 {
        return None;
    }
    let mut sum = vec![0.0f32; dim];
    for v in vecs {
        if v.len() != dim {
            return None;
        }
        for (s, x) in sum.iter_mut().zip(v.iter()) {
            *s += x;
        }
    }
    let n = vecs.len() as f32;
    Some(sum.into_iter().map(|x| x / n).collect())
}

fn classify_from_scores(advisory: f32, diagnostic: f32) -> IntentClass {
    // Require meaningful separation — if they're close, stay ambiguous.
    // Tuned conservatively: suppressing a real diagnostic query is worse than
    // failing to suppress a conversational one (keyword guard handles most of those).
    const ADVISORY_MIN: f32 = 0.72; // minimum score to declare advisory
    const DIAGNOSTIC_MIN: f32 = 0.68; // minimum score to declare diagnostic
    const MIN_GAP: f32 = 0.08; // required margin over the other class

    if advisory >= ADVISORY_MIN && advisory > diagnostic + MIN_GAP {
        IntentClass::Advisory
    } else if diagnostic >= DIAGNOSTIC_MIN && diagnostic > advisory + MIN_GAP {
        IntentClass::Diagnostic
    } else {
        IntentClass::Ambiguous
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 2.0, 3.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-5);
    }

    #[test]
    fn centroid_of_two_identical() {
        let vecs = vec![vec![1.0, 2.0], vec![1.0, 2.0]];
        let c = mean_centroid(&vecs).unwrap();
        assert!((c[0] - 1.0).abs() < 1e-5);
        assert!((c[1] - 2.0).abs() < 1e-5);
    }

    #[test]
    fn classify_from_scores_advisory() {
        assert_eq!(classify_from_scores(0.80, 0.60), IntentClass::Advisory);
    }

    #[test]
    fn classify_from_scores_diagnostic() {
        assert_eq!(classify_from_scores(0.55, 0.78), IntentClass::Diagnostic);
    }

    #[test]
    fn classify_from_scores_ambiguous_close_gap() {
        assert_eq!(classify_from_scores(0.74, 0.70), IntentClass::Ambiguous);
    }

    #[test]
    fn classify_from_scores_ambiguous_low_scores() {
        assert_eq!(classify_from_scores(0.50, 0.40), IntentClass::Ambiguous);
    }
}
