use crate::utils::github as gh;
use tracing::{error, info};

pub fn parse_github_repo_env() -> Option<(String, String)> {
    let repo = std::env::var("FRAMEWORK_CONTROL_UPDATE_REPO").ok()?;
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
        // quiet
        .arg("/qn")
        // no restart
        .arg("/norestart")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
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
    #[cfg(target_os = "macos")]
    let preferred_exts: &[&str] = &[".pkg", ".dmg"];
    #[cfg(all(unix, not(target_os = "macos")))]
    let preferred_exts: &[&str] = &[".deb", ".rpm", ".AppImage"];
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
    #[cfg(not(target_os = "windows"))]
    {
        Err("update apply unsupported on this OS".into())
    }
}
