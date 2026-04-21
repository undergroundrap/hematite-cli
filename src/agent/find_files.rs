use serde_json::Value;
use walkdir::WalkDir;
use crate::agent::fuzzy::fuzzy_match;

/// Precision File Discovery using Fuzzy Matching.
/// Replaces broad grep loops with surgical name-based sightings.
pub async fn find_files_fuzzy(args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing required argument: 'query'".to_string())?;

    let max_results = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let workspace_root = crate::tools::file_ops::workspace_root();
    
    let mut matches = Vec::new();

    for entry in WalkDir::new(&workspace_root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        })
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(&workspace_root).unwrap_or(entry.path());
            let path_str = rel_path.to_string_lossy();
            
            if let Some((_, score)) = fuzzy_match(&path_str, query) {
                matches.push((path_str.into_owned(), score));
            }
        }
    }

    // Sort by score (lower is better)
    matches.sort_by_key(|m| m.1);
    matches.truncate(max_results);

    if matches.is_empty() {
        return Ok(format!("No files found matching '{}'", query));
    }

    let mut output = format!("Found {} matches for '{}':\n", matches.len(), query);
    for (path, score) in matches {
        let confidence = if score <= -100 { "High" } else if score <= 5 { "Moderate" } else { "Low" };
        output.push_str(&format!("- {} (Confidence: {})\n", path, confidence));
    }

    Ok(output)
}
