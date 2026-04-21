use serde_json::Value;

/// Precision File Discovery using Fuzzy Matching.
pub async fn find_files_fuzzy(args: &Value) -> Result<String, String> {
    crate::agent::find_files::find_files_fuzzy(args).await
}
