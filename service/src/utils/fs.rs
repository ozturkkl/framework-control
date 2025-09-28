use tracing::warn;

/// Recursively copy directory contents from src into dst, creating directories as needed and
/// overwriting existing files. Best-effort: logs and continues on individual copy errors.
pub fn copy_dir_replace(src: &std::path::Path, dst: &std::path::Path) {
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) => {
            warn!("read_dir failed for '{}': {}", src.to_string_lossy(), e);
            return;
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let rel = match path.strip_prefix(src) {
            Ok(r) => r,
            Err(_) => continue,
        };
        let dest = dst.join(rel);
        if path.is_dir() {
            let _ = std::fs::create_dir_all(&dest);
            copy_dir_replace(&path, &dest);
        } else {
            if let Some(parent) = dest.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::copy(&path, &dest) {
                warn!(
                    "copy '{}' -> '{}' failed: {}",
                    path.to_string_lossy(),
                    dest.to_string_lossy(),
                    e
                );
            }
        }
    }
}


