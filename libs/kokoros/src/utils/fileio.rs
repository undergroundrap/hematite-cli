use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::io::Read;
use tokio::{fs::File, io::AsyncWriteExt};
use reqwest;

pub async fn download_file_from_url(
    url: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let res = reqwest::get(url).await?;
    let total_size = res
        .content_length()
        .ok_or_else(|| format!("Failed to get content length from '{}'", url))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", path));

    let mut file = File::create(path).await?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    use futures_util::StreamExt;
    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk).await?;
        let new = std::cmp::min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {} to {}", url, path));
    Ok(())
}

pub fn read_json_file(path: &str) -> Result<Value, Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    let json: Value = serde_json::from_str(&content)?;
    Ok(json)
}
