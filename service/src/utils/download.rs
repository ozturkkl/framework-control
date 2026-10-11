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
    while let Some(chunk) = resp.chunk().await.map_err(|e| format!("download read failed: {e}"))? {
        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk).await.map_err(|e| format!("write failed: {e}"))?;
    }
    {
        use tokio::io::AsyncWriteExt;
        file.flush().await.map_err(|e| format!("flush failed: {e}"))?;
    }
    if let Ok(meta) = std::fs::metadata(&dest_file_path) {
        info!("downloaded size: {} bytes", meta.len());
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum ArchiveKind {
    Zip,
    TarGz,
}

/// Returns the archive kind and the file name without its archive extension, or None for non-archives.
fn split_archive_name(filename: &str) -> Option<(ArchiveKind, &str)> {
    let filename_lc = filename.to_ascii_lowercase();
    for (ext, kind) in [
        (".zip", ArchiveKind::Zip),
        (".tar.gz", ArchiveKind::TarGz),
        (".tgz", ArchiveKind::TarGz),
    ] {
        if filename_lc.ends_with(ext) && filename.len() > ext.len() {
            return Some((kind, &filename[..filename.len() - ext.len()]));
        }
    }
    None
}

/// Download to a root directory. If the URL is a .zip or .tar.gz, it will be extracted into a
/// subfolder named after the archive's file stem. Otherwise, the file will be saved in the
/// root directory using the URL's filename.
/// Returns the final path created: directory path for archives, or file path for non-archives.
pub async fn download_to_path(url: &str, root_dir: &str) -> Result<String, String> {
    let root_dir_p = std::path::Path::new(root_dir);
    let _ = std::fs::create_dir_all(root_dir_p);

    // Derive filename from URL (strip query string if present)
    let url_last = url.rsplit('/').next().unwrap_or("download.bin");
    let filename = url_last.split('?').next().unwrap_or(url_last);

    let dest_file = root_dir_p.join(filename);
    let dest_file_s = dest_file.to_string_lossy().to_string();
    download_raw_to_file(url, &dest_file_s).await?;

    let Some((kind, folder_name)) = split_archive_name(filename) else {
        return Ok(dest_file_s);
    };
    let final_dir_s = root_dir_p.join(folder_name).to_string_lossy().to_string();
    match kind {
        ArchiveKind::Zip => {
            crate::utils::extract::extract_zip_to(&dest_file_s, &final_dir_s)
                .map_err(|e| format!("zip extract failed: {e}"))?;
        }
        ArchiveKind::TarGz => {
            crate::utils::extract::extract_tar_gz_to(&dest_file_s, &final_dir_s)
                .await
                .map_err(|e| format!("tar.gz extract failed: {e}"))?;
        }
    }
    std::fs::remove_file(&dest_file_s).map_err(|e| format!("remove downloaded archive failed: {e}"))?;
    Ok(final_dir_s)
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
