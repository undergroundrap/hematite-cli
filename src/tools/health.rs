use chrono;
use serde_json::{json, Value};

pub async fn execute(_args: &Value) -> Result<String, String> {
    let now = chrono::Utc::now();
    let file_count = std::fs::read_dir("src/")
        .map_err(|e| e.to_string())?
        .filter(|e| e.is_ok())
        .count();
    Ok(json!({
        "status": "healthy",
        "engine": "Hematite",
        "version": crate::HEMATITE_VERSION,
        "build": crate::hematite_build_descriptor(),
        "time": format!("{}", now.format("%Y-%m-%d %H:%M:%S UTC")),
        "src_file_count": file_count
    })
    .to_string())
}
