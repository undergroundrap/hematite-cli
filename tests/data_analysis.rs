use std::fs;
use std::path::PathBuf;
use serde_json::json;

#[tokio::test]
async fn test_sql_csv_ingestion_and_query() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let csv_path = workspace.path().join("test.csv");
    fs::write(&csv_path, "id,name,value\n1,alice,10.5\n2,bob,20.0\n3,charlie,30.5").expect("write csv");

    let args = json!({
        "sql": "SELECT count(*) as total, sum(value) as sum_val FROM source",
        "path": csv_path.to_str().unwrap()
    });

    let result = hematite::tools::data_query::query_data(&args).await.expect("query failed");
    assert!(result.contains("total"));
    assert!(result.contains("3"));
    assert!(result.contains("61.0"));
}

#[tokio::test]
async fn test_export_as_table_sqlite() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let db_path = workspace.path().join("out.db");

    let args = json!({
        "items": [
            {"name": "x", "val": 100},
            {"name": "y", "val": 200}
        ],
        "path": db_path.to_str().unwrap(),
        "format": "sqlite"
    });

    let result = hematite::tools::data_query::export_as_table(&args).await.expect("export failed");
    assert!(result.contains("Successfully exported 2 items"));
    assert!(db_path.exists());

    // Verify we can query it back
    let query_args = json!({
        "sql": "SELECT name FROM data WHERE val > 150",
        "path": db_path.to_str().unwrap()
    });
    let query_res = hematite::tools::data_query::query_data(&query_args).await.expect("query failed");
    assert!(query_res.contains("y"));
    assert!(!query_res.contains("x"));
}

#[tokio::test]
async fn test_analyze_trends_logic() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let db_path = workspace.path().join("trends.db");

    // Create a small DB for analysis
    let setup_args = json!({
        "items": [
            {"val": 10}, {"val": 20}, {"val": 30}, {"val": 40}, {"val": 50}
        ],
        "path": db_path.to_str().unwrap(),
        "format": "sqlite"
    });
    hematite::tools::data_query::export_as_table(&setup_args).await.expect("setup failed");

    let args = json!({
        "sql": "SELECT val FROM data",
        "path": db_path.to_str().unwrap()
    });

    let result = hematite::tools::data_query::analyze_trends(&args).await.expect("analysis failed");
    assert!(result.contains("Mean:   30.0000"));
    assert!(result.contains("Distribution (ASCII Histogram)"));
    assert!(result.contains("█"));
}
