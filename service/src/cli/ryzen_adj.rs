use crate::utils::{download as dl, github as gh};
use tokio::process::Command;
use tracing::{error, info, warn};
use which::which;

/// Thin wrapper around the `ryzenadj` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
#[derive(Clone)]
pub struct RyzenAdj {
    pub(crate) path: String,
}

impl RyzenAdj {
    pub async fn new() -> Result<Self, String> {
        let path = resolve_ryzenadj().await?;
        info!("ryzenadj resolved at: {}", path);
        Ok(Self { path })
    }

    /// Set TDP by applying stapm/fast/slow limits equally (expects watts)
    pub async fn set_tdp_watts(&self, watts: u32) -> Result<(), String> {
        let mw = watts.saturating_mul(1000).to_string();
        let _ = self
            .run(&[
                "--stapm-limit",
                &mw,
                "--fast-limit",
                &mw,
                "--slow-limit",
                &mw,
            ])
            .await?;
        Ok(())
    }

    /// Set thermal limit (Tctl) in degrees Celsius
    pub async fn set_thermal_limit_c(&self, celsius: u32) -> Result<(), String> {
        let _ = self.run(&["--tctl-temp", &celsius.to_string()]).await?;
        Ok(())
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        use tokio::time::{timeout, Duration};
        let args: Vec<&str> = {
            let mut v: Vec<&str> = args.to_vec();
            let has_dump = v.iter().any(|a| a.eq_ignore_ascii_case("--dump-table"));
            if !has_dump {
                v.push("--dump-table");
            }
            v
        };
        let child = Command::new(&self.path)
            .args(&args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;
        let output = timeout(Duration::from_secs(5), child.wait_with_output())
            .await
            .map_err(|_| "ryzenadj timed out".to_string())
            .and_then(|res| res.map_err(|e| format!("wait failed: {e}")))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "exit {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

async fn resolve_ryzenadj() -> Result<String, String> {
    // Prefer alongside the running service binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = if cfg!(windows) {
                dir.join("ryzenadj/ryzenadj.exe")
            } else {
                dir.join("ryzenadj/ryzenadj")
            };
            if candidate.exists() {
                if let Some(s) = candidate.to_str() {
                    return Ok(s.to_string());
                }
            }
            // Also allow a root-level binary next to the service
            let root_candidate = if cfg!(windows) {
                dir.join("ryzenadj.exe")
            } else {
                dir.join("ryzenadj")
            };
            if root_candidate.exists() {
                if let Some(s) = root_candidate.to_str() {
                    return Ok(s.to_string());
                }
            }
        }
    }
    if let Ok(p) = std::env::var("RYZENADJ_PATH") {
        let path = std::path::Path::new(&p);
        if path.exists() {
            return Ok(p);
        }
    }

    if let Ok(p) = which("ryzenadj") {
        return Ok(p.to_string_lossy().to_string());
    }
    if let Ok(p) = which("ryzenadj.exe") {
        return Ok(p.to_string_lossy().to_string());
    }

    Err("ryzenadj not found".into())
}

/// Resolve ryzenadj, attempting installation if not present.
pub async fn resolve_or_install_ryzenadj() -> Result<RyzenAdj, String> {
    // 1) Try resolve immediately
    if let Ok(cli) = RyzenAdj::new().await {
        return Ok(cli);
    }

    // 2) Try direct download once from GitHub Releases
    if let Err(err) = attempt_install_via_direct_download().await {
        warn!("ryzenadj direct download failed: {}", err);
    }

    // 3) Final resolve attempt
    match RyzenAdj::new().await {
        Ok(cli) => Ok(cli),
        Err(e) => {
            error!("ryzenadj not found after attempted install: {}", e);
            Err(e)
        }
    }
}

/// Fallback: direct download of ryzenadj from GitHub Releases (Windows/Linux)
pub async fn attempt_install_via_direct_download() -> Result<(), String> {
    // Always download next to the service binary to avoid hardcoded system paths
    let base_dir = match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        Some(p) => p,
        None => return Err("could not resolve service directory for direct download".into()),
    };
    #[cfg(target_os = "windows")]
    let ext: &str = ".exe";
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let ext: &str = "";
    let filename = format!("ryzenadj{}", ext);

    // Try to find a direct .exe (Windows) or bare binary asset
    let url = gh::get_latest_release_url_ending_with("FlyGoat", "RyzenAdj", &[filename.as_str()])
        .await
        .map_err(|e| format!("failed to resolve ryzenadj asset: {e}"))?
        .ok_or_else(|| "ryzenadj asset not found in latest release".to_string())?;
    info!(
        "Attempting direct download of ryzenadj into '{}' from '{}'",
        base_dir.to_string_lossy(),
        url
    );
    let final_path = dl::download_to_path(&url, &base_dir.to_string_lossy().to_string()).await?;

    if let Ok(meta) = std::fs::metadata(&final_path) {
        info!("ryzenadj downloaded size: {} bytes", meta.len());
    }
    // If we extracted a directory, normalize its name to a stable folder: "ryzenadj"
    let final_p = std::path::Path::new(&final_path);
    if final_p.is_dir() {
        let stable_dir = base_dir.join("ryzenadj");
        if stable_dir != final_p {
            // Remove any previous directory and move the new one into place
            if stable_dir.exists() {
                let _ = std::fs::remove_dir_all(&stable_dir);
            }
            std::fs::rename(&final_p, &stable_dir)
                .map_err(|e| format!("failed to move install dir into stable location: {e}"))?;
        }
    }
    Ok(())
}
