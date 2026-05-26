use hmac::{Hmac, Mac};
use md5::Digest as _;

type HmacSha256 = Hmac<sha2::Sha256>;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("sha256");

    match action {
        "md5" => hash_md5(args),
        "sha256" => hash_sha256(args),
        "sha512" => hash_sha512(args),
        "hmac-sha256" => hmac_sha256(args),
        "all" => hash_all(args),
        other => Err(format!(
            "hash_tools: unknown action '{other}'. Valid: md5, sha256, sha512, hmac-sha256, all"
        )),
    }
}

fn get_bytes(args: &serde_json::Value) -> Result<(Vec<u8>, String), String> {
    if let Some(s) = args.get("input").and_then(|v| v.as_str()) {
        return Ok((s.as_bytes().to_vec(), format!("\"{}\"", truncate_label(s))));
    }
    if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
        let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
            std::path::PathBuf::from(r)
        } else {
            crate::tools::file_ops::workspace_root()
        };
        let path = root.join(file);
        let bytes =
            std::fs::read(&path).map_err(|e| format!("hash_tools: cannot read '{file}': {e}"))?;
        let label = format!("{file} ({} bytes)", bytes.len());
        return Ok((bytes, label));
    }
    Err("hash_tools: provide 'input' (inline string) or 'file' (path)".into())
}

fn truncate_label(s: &str) -> String {
    if s.len() > 40 {
        format!("{}...", &s[..40])
    } else {
        s.to_string()
    }
}

fn hash_md5(args: &serde_json::Value) -> Result<String, String> {
    let (bytes, label) = get_bytes(args)?;
    let mut h = md5::Md5::new();
    h.update(&bytes);
    let hex = format!("{:x}", h.finalize());
    Ok(format!(
        "MD5\n{}\nInput : {label}\nDigest: {hex}",
        "─".repeat(50)
    ))
}

fn hash_sha256(args: &serde_json::Value) -> Result<String, String> {
    let (bytes, label) = get_bytes(args)?;
    let mut h = sha2::Sha256::new();
    h.update(&bytes);
    let hex = format!("{:x}", h.finalize());
    Ok(format!(
        "SHA-256\n{}\nInput : {label}\nDigest: {hex}",
        "─".repeat(50)
    ))
}

fn hash_sha512(args: &serde_json::Value) -> Result<String, String> {
    let (bytes, label) = get_bytes(args)?;
    let mut h = sha2::Sha512::new();
    h.update(&bytes);
    let hex = format!("{:x}", h.finalize());
    Ok(format!(
        "SHA-512\n{}\nInput : {label}\nDigest: {hex}",
        "─".repeat(50)
    ))
}

fn hmac_sha256(args: &serde_json::Value) -> Result<String, String> {
    let (bytes, label) = get_bytes(args)?;
    let key = args
        .get("key")
        .and_then(|v| v.as_str())
        .ok_or("hash_tools hmac-sha256: 'key' is required")?;

    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| format!("hash_tools: HMAC key error: {e}"))?;
    mac.update(&bytes);
    let hex = format!("{:x}", mac.finalize().into_bytes());
    Ok(format!(
        "HMAC-SHA256\n{}\nInput : {label}\nKey   : \"{}\"\nDigest: {hex}",
        "─".repeat(50),
        truncate_label(key)
    ))
}

fn hash_all(args: &serde_json::Value) -> Result<String, String> {
    let (bytes, label) = get_bytes(args)?;

    let mut md5h = md5::Md5::new();
    md5h.update(&bytes);
    let md5_hex = format!("{:x}", md5h.finalize());

    let mut sha256h = sha2::Sha256::new();
    sha256h.update(&bytes);
    let sha256_hex = format!("{:x}", sha256h.finalize());

    let mut sha512h = sha2::Sha512::new();
    sha512h.update(&bytes);
    let sha512_hex = format!("{:x}", sha512h.finalize());

    Ok(format!(
        "HASH ALL\n{}\nInput  : {label}\n\nMD5    : {md5_hex}\nSHA-256: {sha256_hex}\nSHA-512: {sha512_hex}",
        "─".repeat(50)
    ))
}
