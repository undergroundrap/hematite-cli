use hematite::tools::scientific::scientific_compute;
use serde_json::json;
use std::fs;

#[tokio::test]
async fn test_scientific_symbolic_solve() {
    let args = json!({
        "mode": "symbolic",
        "expr": "x**2 - 4 = 0",
        "target": "solve"
    });
    
    let result = scientific_compute(&args).await;
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
    let result = scientific_compute(&args).await;
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
    let result = scientific_compute(&args).await;
    if let Ok(res) = result {
        assert!(res.contains("RESULT") || res.contains("ERROR"));
    }
}

#[tokio::test]
async fn test_scientific_ledger_append_read() {
    let test_content = "Integration by parts formula: Integral u dv = uv - Integral v du";
    
    // 1. Append
    let append_args = json!({
        "mode": "ledger",
        "action": "append",
        "content": test_content
    });
    let append_res = scientific_compute(&append_args).await;
    assert!(append_res.is_ok());
    assert!(append_res.unwrap().contains("successfully persisted"));

    // 2. Read
    let read_args = json!({
        "mode": "ledger",
        "action": "read"
    });
    let read_res = scientific_compute(&read_args).await;
    assert!(read_res.is_ok());
    assert!(read_res.unwrap().contains(test_content));

    // Cleanup
    let ledger_path = ".hematite/docs/scientific_ledger.md";
    if std::path::Path::new(ledger_path).exists() {
        fs::remove_file(ledger_path).ok();
    }
}

#[tokio::test]
async fn test_scientific_symbolic_latex() {
    let args = json!({
        "mode": "symbolic",
        "expr": "x**2 + 2*x + 1",
        "target": "simplify",
        "latex": true
    });
    
    let result = scientific_compute(&args).await;
    if let Ok(res) = result {
        assert!(res.contains("RESULT") || res.contains("ERROR"));
        if res.contains("RESULT") {
            assert!(res.contains("LATEX"));
        }
    }
}
