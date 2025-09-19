use tracing::info;

pub async fn download_to_file(url: &str, dest_path: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))?;
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    if let Some(parent) = std::path::Path::new(dest_path).parent() { let _ = std::fs::create_dir_all(parent); }
    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|e| format!("failed to create dest file: {e}"))?;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download read failed: {e}"))?
    {
        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("write failed: {e}"))?;
    }
    {
        use tokio::io::AsyncWriteExt;
        file.flush()
            .await
            .map_err(|e| format!("flush failed: {e}"))?;
    }
    if let Ok(meta) = std::fs::metadata(&dest_path) { info!("downloaded size: {} bytes", meta.len()); }
    Ok(())
}

