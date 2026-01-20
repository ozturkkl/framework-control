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



/// Detect the actual user's home directory when running as root (Linux only)
/// Checks SUDO_USER, active sessions in /run/user/, and falls back to HOME
#[cfg(target_os = "linux")]
pub fn detect_user_home() -> Option<String> {
    // Check if running via sudo
    if let Ok(sudo_user) = std::env::var("SUDO_USER") {
        return Some(format!("/home/{}", sudo_user));
    }

    // Scan /run/user/ for active sessions with UID >= 1000
    if let Ok(entries) = std::fs::read_dir("/run/user") {
        for entry in entries.flatten() {
            let file_name = match entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };

            let uid = match file_name.parse::<u32>() {
                Ok(uid) if uid >= 1000 => uid,
                _ => continue,
            };

            // Try to get username from UID
            let output = match std::process::Command::new("id")
                .arg("-un")
                .arg(uid.to_string())
                .output()
            {
                Ok(out) if out.status.success() => out,
                _ => continue,
            };

            let username = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !username.is_empty() {
                return Some(format!("/home/{}", username));
            }
        }
    }

    // Fallback to HOME
    std::env::var("HOME").ok()
}
