use tokio::process::Command;
use tracing::{warn};

#[cfg(target_os = "windows")]
fn find_winget_path() -> Option<String> {
    if let Ok(windir) = std::env::var("WINDIR") {
        let sys32 = std::path::Path::new(&windir)
            .join("System32")
            .join("winget.exe");
        if sys32.exists() {
            return sys32.to_str().map(|s| s.to_string());
        }
    }

    let windows_apps_root = std::env::var("ProgramFiles")
        .ok()
        .map(|pf| std::path::Path::new(&pf).join("WindowsApps"))
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\\Program Files\\WindowsApps"));
    if let Ok(entries) = std::fs::read_dir(&windows_apps_root) {
        let mut dirs: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        dirs.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
        for d in dirs {
            let name = d.file_name();
            if let Some(s) = name.to_str() {
                if s.starts_with("Microsoft.DesktopAppInstaller_") {
                    let winget = d.path().join("winget.exe");
                    if winget.exists() {
                        if let Some(p) = winget.to_str() {
                            return Some(p.to_string());
                        }
                    }
                    let alt = d.path().join("AppInstallerCLI.exe");
                    if alt.exists() {
                        if let Some(p) = alt.to_str() {
                            return Some(p.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn find_winget_path() -> Option<String> { None }

#[cfg(target_os = "windows")]
pub async fn try_winget_install_package(package_id: &str, location: Option<&str>) -> Result<(), String> {
    use tokio::time::{timeout, Duration};
    let winget_path = find_winget_path();
    if winget_path.is_none() { warn!("winget.exe not resolved explicitly; relying on PATH"); }

    // Determine install location (default to directory of current executable)
    let install_location_owned: String = match location {
        Some(loc) => loc.to_string(),
        None => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .and_then(|d| d.to_str().map(|s| s.to_string()))
            .ok_or_else(|| "could not resolve service directory for winget install".to_string())?,
    };

    let child = if let Some(winget_exe) = &winget_path {
        Command::new(winget_exe)
            .args([
                "install",
                package_id,
                "--accept-source-agreements",
                "--accept-package-agreements",
                "--silent",
                "--disable-interactivity",
                "--scope",
                "machine",
                "--location",
                &install_location_owned,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn winget: {e}"))?
    } else {
        let install_cmd = format!(
            "winget install {} --accept-source-agreements --accept-package-agreements --silent --disable-interactivity --scope machine --location '{}'",
            package_id,
            install_location_owned.replace("'", "''")
        );
        Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &install_cmd])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn powershell: {e}"))?
    };

    let output = timeout(Duration::from_secs(300), child.wait_with_output())
        .await
        .map_err(|_| "winget install timed out".to_string())
        .and_then(|r| r.map_err(|e| format!("winget wait failed: {e}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("winget install failed: {}", output.status))
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn try_winget_install_package(_package_id: &str, _location: Option<&str>) -> Result<(), String> {
    Err("winget installation is only supported on Windows".into())
}


