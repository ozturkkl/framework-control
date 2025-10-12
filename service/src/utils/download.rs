use tracing::info;

async fn download_raw_to_file(url: &str, dest_file_path: &str) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(dest_file_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

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

    let mut file = tokio::fs::File::create(&dest_file_path)
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
    if let Ok(meta) = std::fs::metadata(&dest_file_path) {
        info!("downloaded size: {} bytes", meta.len());
    }
    Ok(())
}

/// Download to a root directory. If the URL is a .zip, it will be extracted into a
/// subfolder named after the zip's file stem. Otherwise, the file will be saved in the
/// root directory using the URL's filename.
/// Returns the final path created: directory path for zips, or file path for non-zips.
pub async fn download_to_path(url: &str, root_dir: &str) -> Result<String, String> {
    let is_zip = url.to_ascii_lowercase().ends_with(".zip");

    // Ensure root directory exists
    let root_dir_p = std::path::Path::new(root_dir);
    let _ = std::fs::create_dir_all(&root_dir_p);

    // Derive filename from URL (strip query string if present)
    let url_last = url.rsplit('/').next().unwrap_or("download.bin");
    let filename = url_last.split('?').next().unwrap_or(url_last);

    if is_zip {
        // Determine folder name from filename without .zip
        let folder_name = if filename.to_ascii_lowercase().ends_with(".zip") && filename.len() > 4 {
            &filename[..filename.len() - 4]
        } else {
            filename
        };
        let final_dir = root_dir_p.join(folder_name);
        if let Some(parent) = final_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Download zip next to the final directory using original zip name
        let tmp_zip_path = root_dir_p.join(filename);
        let tmp_zip_s = tmp_zip_path.to_string_lossy().to_string();
        download_raw_to_file(url, &tmp_zip_s).await?;

        crate::utils::zip_extract::extract_zip_to(
            &tmp_zip_s,
            &final_dir.to_string_lossy().to_string(),
        )
        .map_err(|e| format!("zip extract failed: {e}"))?;
        if let Ok(meta) = std::fs::metadata(&tmp_zip_s) {
            info!("zip downloaded size: {} bytes", meta.len());
        }
        std::fs::remove_file(&tmp_zip_s).map_err(|e| format!("remove temp zip failed: {e}"))?;
        return Ok(final_dir.to_string_lossy().to_string());
    }

    let dest_file = root_dir_p.join(filename);
    let dest_file_s = dest_file.to_string_lossy().to_string();
    download_raw_to_file(url, &dest_file_s).await?;
    Ok(dest_file_s)
}
