use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

/// Extract a tar.gz archive to a target directory using system tar command
pub async fn extract_tar_gz_to<P: AsRef<Path>>(tar_path: P, target_dir: P) -> Result<(), String> {
    let tar_path = tar_path.as_ref();
    let target_dir = target_dir.as_ref();

    // Ensure target directory exists
    std::fs::create_dir_all(target_dir)
        .map_err(|e| format!("failed to create target dir: {}", e))?;

    // Use system tar command
    let status = tokio::process::Command::new("tar")
        .arg("-xzf")
        .arg(tar_path)
        .arg("-C")
        .arg(target_dir)
        .status()
        .await
        .map_err(|e| format!("failed to run tar command: {}", e))?;

    if !status.success() {
        return Err(format!("tar extraction failed with status: {}", status));
    }

    Ok(())
}

pub fn extract_zip_to<P: AsRef<Path>>(zip_path: P, target_dir: P) -> Result<Vec<PathBuf>, String> {
    let zip_path = zip_path.as_ref();
    let target_dir = target_dir.as_ref();
    let file = File::open(zip_path).map_err(|e| format!("open zip failed: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(BufReader::new(file)).map_err(|e| format!("zip open failed: {e}"))?;
    let mut extracted: Vec<PathBuf> = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("zip entry failed: {e}"))?;
        let outpath = target_dir.join(file.mangled_name());
        if file.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("mkdir failed: {e}"))?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| format!("create failed: {e}"))?;
            let mut buf = Vec::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("read failed: {e}"))?;
            outfile
                .write_all(&buf)
                .map_err(|e| format!("write failed: {e}"))?;
        }
        extracted.push(outpath);
    }
    Ok(extracted)
}

/// Download an archive (zip or tar.gz) to a temp dir and check whether it contains a file ending with any preferred suffixes
pub async fn archive_contains_any_suffix(
    url: &str,
    preferred_suffixes: &[&str],
) -> Result<bool, String> {
    // Use a URL-based hash to isolate temp artifacts per asset and ensure idempotency
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    url.hash(&mut hasher);
    let h = hasher.finish();

    let executable_path =
        std::env::current_exe().map_err(|e| format!("get current executable path failed: {e}"))?;
    let executable_dir = executable_path
        .parent()
        .unwrap_or(std::path::Path::new("."));
    let tmp_root = executable_dir.join("fc_zip_peek");
    let _ = std::fs::create_dir_all(&tmp_root);
    let tmp_dir = tmp_root.join(format!("{}", h));
    // Clean any previous contents for this URL
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let _ = std::fs::create_dir_all(&tmp_dir);

    let extract_dir_s =
        crate::utils::download::download_to_path(url, &tmp_dir.to_string_lossy().to_string())
            .await?;
    let extract_dir = std::path::Path::new(&extract_dir_s).to_path_buf();
    // walk and find preferred
    let mut stack = vec![extract_dir.clone()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| e.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                let name_lc = name.to_ascii_lowercase();
                if preferred_suffixes
                    .iter()
                    .any(|s| name_lc.ends_with(&s.to_ascii_lowercase()))
                {
                    std::fs::remove_dir_all(&tmp_dir).map_err(|e| format!("remove temp dir failed: {e}"))?;
                    return Ok(true);
                }
            }
        }
    }
    std::fs::remove_dir_all(&tmp_dir).map_err(|e| format!("remove temp dir failed: {e}"))?;
    Ok(false)
}
