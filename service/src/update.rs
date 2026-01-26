use crate::utils::github as gh;
use tracing::{error, info};

pub fn parse_github_repo_env() -> Option<(String, String)> {
    let repo = std::env::var("FRAMEWORK_CONTROL_UPDATE_REPO")
        .ok()
        .or_else(|| option_env!("FRAMEWORK_CONTROL_UPDATE_REPO").map(String::from))?;
    if repo.contains('/') && !repo.contains("github.com") {
        let mut it = repo.splitn(2, '/');
        Some((
            it.next().unwrap_or("").to_string(),
            it.next().unwrap_or("").to_string(),
        ))
    } else {
        let parts: Vec<&str> = repo.split('/').collect();
        let owner = parts
            .get(parts.len().saturating_sub(2))
            .cloned()
            .unwrap_or("");
        let name = parts
            .get(parts.len().saturating_sub(1))
            .cloned()
            .unwrap_or("");
        if owner.is_empty() || name.is_empty() {
            None
        } else {
            Some((owner.to_string(), name.to_string()))
        }
    }
}

pub async fn get_current_and_latest() -> Result<(String, String), String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let current_trimmed = current.trim().to_string();
    if current_trimmed.is_empty() {
        return Err("current version missing".into());
    }
    let Some((owner, name)) = parse_github_repo_env() else {
        return Err("FRAMEWORK_CONTROL_UPDATE_REPO not set".into());
    };
    let latest_opt = gh::get_latest_release_version_tag(&owner, &name).await?;
    let latest = latest_opt.ok_or_else(|| "latest version missing".to_string())?;
    Ok((current_trimmed, latest))
}

#[cfg(target_os = "windows")]
async fn spawn_msiexec_install(msi_url: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("framework-control-update.msi");
    let resp = reqwest::get(msi_url.to_string())
        .await
        .map_err(|_| "failed to download msi".to_string())?;
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| "failed to read msi bytes".to_string())?;
    std::fs::write(&tmp, &bytes).map_err(|_| "failed to write msi".to_string())?;
    tokio::process::Command::new("msiexec")
        // install
        .arg("/i")
        .arg(tmp.as_os_str())
        // preserve user's original shortcut choice by installing only core feature
        .arg("ADDLOCAL=Binaries")
        // quiet
        .arg("/qn")
        // no restart
        .arg("/norestart")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
async fn extract_and_replace_binary(tarball_url: &str) -> Result<(), String> {
    let tmp_dir = std::env::temp_dir().join("framework-control-update");
    std::fs::create_dir_all(&tmp_dir).map_err(|e| format!("failed to create temp dir: {}", e))?;

    // Download and extract tarball (download_to_path handles tar.gz extraction automatically)
    let extracted_dir = crate::utils::download::download_to_path(
        tarball_url,
        &tmp_dir.to_string_lossy().to_string(),
    )
    .await?;

    // Find the extracted binary
    let extracted_binary = std::path::Path::new(&extracted_dir).join("framework-control");
    if !extracted_binary.exists() {
        return Err("extracted binary not found".to_string());
    }

    // Get the current binary path
    let current_exe = std::env::current_exe()
        .map_err(|e| format!("failed to get current exe path: {}", e))?;

    // Create path for new binary (same directory as current)
    let current_dir = current_exe.parent()
        .ok_or_else(|| "failed to get current binary directory".to_string())?;
    let new_binary = current_dir.join("framework-control.new");

    info!("Installing update to {}", new_binary.display());

    // Copy to .new file
    std::fs::copy(&extracted_binary, &new_binary)
        .map_err(|e| {
            format!(
                "failed to copy new binary (may need root/sudo permissions): {}",
                e
            )
        })?;

    // Make it executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&new_binary)
            .map_err(|e| format!("failed to get binary metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&new_binary, perms)
            .map_err(|e| format!("failed to set binary permissions: {}", e))?;
    }

    // Clean up temp files
    let _ = std::fs::remove_dir_all(&tmp_dir);

    info!("Binary staged successfully, scheduling atomic replacement and restart...");

    // Spawn a background task to atomically replace and restart
    let current_exe_clone = current_exe.clone();
    let new_binary_clone = new_binary.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Atomic rename (this works even if the old binary is running)
        if let Err(e) = std::fs::rename(&new_binary_clone, &current_exe_clone) {
            error!("Failed to replace binary: {}", e);
            return;
        }

        info!("Binary replaced successfully, restarting service...");

        match tokio::process::Command::new("systemctl")
            .arg("restart")
            .arg("framework-control")
            .status()
            .await
        {
            Ok(status) if status.success() => {
                info!("Service restart initiated");
            }
            Ok(status) => {
                info!(
                    "Service restart failed (exit {}). Manual restart required: \
                     sudo systemctl restart framework-control",
                    status
                );
            }
            Err(e) => {
                info!(
                    "Could not restart service ({}). Manual restart required: \
                     sudo systemctl restart framework-control",
                    e
                );
            }
        }
    });

    Ok(())
}

/// Checks for a newer release and, if found, downloads and starts installation.
/// Returns Ok(true) if an update was initiated, Ok(false) if no update needed.
pub async fn check_and_apply_now() -> Result<bool, String> {
    let Some((owner, name)) = parse_github_repo_env() else {
        return Err("FRAMEWORK_CONTROL_UPDATE_REPO not set".into());
    };
    let (current, latest) = get_current_and_latest().await?;
    if latest <= current {
        return Ok(false);
    }
    #[cfg(target_os = "windows")]
    let preferred_exts: &[&str] = &[".msi"];
    #[cfg(target_os = "linux")]
    let preferred_exts: &[&str] = &[".tar.gz"];
    let Some(installer_url) = gh::get_latest_release_url_ending_with(&owner, &name, preferred_exts)
        .await
        .map_err(|e| {
            error!("update: fetch assets failed: {}", e);
            e
        })?
    else {
        error!("update: no installer asset in latest release");
        return Err("installer asset not found".into());
    };

    #[cfg(target_os = "windows")]
    {
        match spawn_msiexec_install(&installer_url).await {
            Ok(_) => {
                info!("msiexec started for update");
                Ok(true)
            }
            Err(e) => {
                error!("failed to start msiexec: {}", e);
                Err(e)
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        match extract_and_replace_binary(&installer_url).await {
            Ok(_) => {
                info!("Linux binary updated successfully");
                Ok(true)
            }
            Err(e) => {
                error!("failed to update Linux binary: {}", e);
                Err(e)
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Err("update apply unsupported on this OS".into())
    }
}
