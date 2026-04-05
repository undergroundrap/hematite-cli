use serde_json::Value;
use base64::prelude::*;
use std::path::Path;
use crate::agent::inference::{InferenceEngine, ChatMessage};

pub async fn vision_analyze(
    engine: &InferenceEngine,
    args: &Value,
) -> Result<String, String> {
    let path_str = args.get("path").and_then(|v| v.as_str())
        .ok_or("Missing parameter: path")?;
    let prompt = args.get("prompt").and_then(|v| v.as_str())
        .ok_or("Missing parameter: prompt")?;

    let path = Path::new(path_str);
    if !path.exists() {
        return Err(format!("File not found: {}", path_str));
    }

    let data = std::fs::read(path).map_err(|e| format!("Failed to read image: {}", e))?;
    let b64 = BASE64_STANDARD.encode(data);
    
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png");
    let mime = match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/png",
    };

    let url = format!("data:{};base64,{}", mime, b64);
    
    let messages = vec![
        ChatMessage::system("You are a vision-capable technical assistant. Analyze the provided image (likely a screenshot, diagram, or UI mockup) and provide a concise technical summary or answer the specific query."),
        ChatMessage::user_with_image(prompt, &url),
    ];

    // Use the main engine but with tools disabled for the vision-pass sub-call.
    let (text, _, _, _) = engine.call_with_tools(&messages, &[], None).await?;
    
    Ok(text.unwrap_or_else(|| "The vision model returned an empty response.".to_string()))
}
