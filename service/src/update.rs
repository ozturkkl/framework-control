use serde_json::Value;
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
        let owner = parts.get(parts.len().saturating_sub(2)).cloned().unwrap_or("");
        let name = parts.get(parts.len().saturating_sub(1)).cloned().unwrap_or("");
        if owner.is_empty() || name.is_empty() {
            None
        } else {
            Some((owner.to_string(), name.to_string()))
        }
    }
}

pub async fn fetch_latest_release(owner: &str, name: &str) -> Result<Value, String> {
    let api = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, name);
    let resp = reqwest::Client::new()
        .get(api)
        .header("user-agent", "framework-control-service")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str::<Value>(&text).map_err(|e| e.to_string())
}

pub fn extract_latest_version_tag(parsed: &Value) -> Option<String> {
    let tag = parsed.get("tag_name").and_then(|v| v.as_str())?;
    let v = tag.trim_start_matches('v').to_string();
    if v.is_empty() { None } else { Some(v) }
}

pub fn find_installer_url(parsed: &Value) -> Option<String> {
    let assets = parsed.get("assets")?.as_array()?.clone();
    #[cfg(target_os = "windows")]
    let preferred_exts: &[&str] = &[".msi"];
    #[cfg(target_os = "macos")]
    let preferred_exts: &[&str] = &[".pkg", ".dmg"];
    #[cfg(all(unix, not(target_os = "macos")))]
    let preferred_exts: &[&str] = &[".deb", ".rpm", ".AppImage"];

    assets.iter().find_map(|a| {
        let name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let matches = preferred_exts.iter().any(|ext| name.ends_with(ext));
        if matches {
            a.get("browser_download_url").and_then(|v| v.as_str()).map(|s| s.to_string())
        } else {
            None
        }
    })
}

pub async fn get_current_and_latest() -> Result<(String, Option<String>), String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let Some((owner, name)) = parse_github_repo_env() else {
        return Ok((current, None));
    };
    let parsed = fetch_latest_release(&owner, &name).await?;
    let latest = extract_latest_version_tag(&parsed);
    Ok((current, latest))
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
    let current = env!("CARGO_PKG_VERSION").to_string();
    let parsed = fetch_latest_release(&owner, &name).await.map_err(|e| {
        error!("update: fetch latest failed: {}", e);
        e
    })?;
    let latest = match extract_latest_version_tag(&parsed) {
        Some(v) => v,
        None => return Ok(false),
    };
    if latest <= current {
        return Ok(false);
    }
    let Some(installer_url) = find_installer_url(&parsed) else {
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
