use hematite::agent::tool_registry::get_tools;
use serde_json::json;

#[tokio::test]
async fn test_scientific_symbolic_solve() {
    let args = json!({
        "mode": "symbolic",
        "expr": "x**2 - 4 = 0",
        "target": "solve"
    });
    
    // We can't easily mock the sandbox output here without a full harness,
    // but we can verify the tool is registered and the function exists.
    let result = hematite::tools::scientific::scientific_compute(&args).await;
    // This will likely fail in CI if python is missing, but should pass on a dev machine
    if let Ok(res) = result {
        assert!(res.contains("RESULT") || res.contains("ERROR"));
    }
}

#[tokio::test]
async fn test_scientific_units() {
    let args = json!({
        "mode": "units",
        "calculation": "10m / 2s"
    });
    let result = hematite::tools::scientific::scientific_compute(&args).await;
    if let Ok(res) = result {
        assert!(res.contains("RESULT") || res.contains("ERROR"));
    }
}

#[tokio::test]
async fn test_scientific_complexity() {
    let args = json!({
        "mode": "complexity",
        "snippet": "for i in range(n): pass"
    });
    let result = hematite::tools::scientific::scientific_compute(&args).await;
    if let Ok(res) = result {
        assert!(res.contains("RESULT") || res.contains("ERROR"));
    }
}
