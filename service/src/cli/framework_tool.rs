use super::framework_tool_parser::{
    parse_power, parse_thermal, parse_versions, PowerParsed, ThermalParsed, VersionsParsed,
};
use crate::utils::{download as dl, github as gh, wget as wg};
use tokio::process::Command;
use tracing::{error, info, warn};
use which::which;

/// Thin wrapper around the `framework_tool` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
#[derive(Clone)]
pub struct FrameworkTool {
    pub(crate) path: String,
}

impl FrameworkTool {
    pub async fn new() -> Result<Self, String> {
        let path = resolve_framework_tool().await?;
        info!("framework_tool resolved at: {}", path);
        Ok(Self { path })
    }

    pub async fn power(&self) -> Result<PowerParsed, String> {
        let out = self.run(&["--power"]).await?;
        Ok(parse_power(&out))
    }

    pub async fn thermal(&self) -> Result<ThermalParsed, String> {
        let out = self.run(&["--thermal"]).await?;
        Ok(parse_thermal(&out))
    }

    pub async fn versions(&self) -> Result<VersionsParsed, String> {
        let out = self.run(&["--versions"]).await?;
        Ok(parse_versions(&out))
    }

    pub async fn set_fan_duty(&self, percent: u32, fan_index: Option<u32>) -> Result<(), String> {
        let percent_s = percent.to_string();
        let fan_idx_s = fan_index.map(|idx| idx.to_string());
        let mut args: Vec<&str> = vec!["--fansetduty"];
        if let Some(ref idxs) = fan_idx_s {
            args.push(idxs.as_str());
        }
        args.push(percent_s.as_str());
        let _ = self.run(&args).await?;
        Ok(())
    }

    pub async fn autofanctrl(&self) -> Result<(), String> {
        let _ = self.run(&["--autofanctrl"]).await?;
        Ok(())
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        use tokio::time::{timeout, Duration};
        let child = Command::new(&self.path)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?;
        let output = timeout(Duration::from_secs(5), child.wait_with_output())
            .await
            .map_err(|_| "framework_tool timed out".to_string())
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

async fn resolve_framework_tool() -> Result<String, String> {
    // Prefer alongside the running service binary
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = if cfg!(windows) {
                dir.join("framework_tool.exe")
            } else {
                dir.join("framework_tool")
            };
            if candidate.exists() {
                if let Some(s) = candidate.to_str() {
                    return Ok(s.to_string());
                }
            }
        }
    }
    if let Ok(p) = std::env::var("FRAMEWORK_TOOL_PATH") {
        let path = std::path::Path::new(&p);
        if path.exists() {
            return Ok(p);
        }
    }

    if let Ok(p) = which("framework_tool") {
        return Ok(p.to_string_lossy().to_string());
    }
    if let Ok(p) = which("framework_tool.exe") {
        return Ok(p.to_string_lossy().to_string());
    }

    Err("framework_tool not found. Please install via winget: winget install FrameworkComputer.framework_tool".into())
}

/// Resolve framework_tool, attempting installation if not present.
pub async fn resolve_or_install() -> Result<FrameworkTool, String> {
    // 1) Try resolve immediately
    if let Ok(cli) = FrameworkTool::new().await {
        return Ok(cli);
    }

    // 2) Try winget install once
    if let Err(err) = wg::try_winget_install_package("FrameworkComputer.framework_tool", None).await
    {
        warn!("winget automatic install failed: {}", err);
    }

    // 3) Try resolve again
    if let Ok(cli) = FrameworkTool::new().await {
        return Ok(cli);
    }

    // 4) Try direct download once
    if let Err(err) = attempt_install_via_direct_download().await {
        warn!("direct download fallback failed: {}", err);
    }

    // 5) Final resolve attempt
    match FrameworkTool::new().await {
        Ok(cli) => Ok(cli),
        Err(e) => {
            error!("framework_tool not found after attempted installs: {}", e);
            Err(e)
        }
    }
}

/// Fallback: cross-platform direct download of framework_tool from GitHub Releases
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
    let filename = format!("framework_tool{}", ext);
    let url = gh::get_latest_release_url_ending_with(
        "FrameworkComputer",
        "framework-system",
        &[filename.as_str()],
    )
    .await
    .map_err(|e| format!("failed to resolve framework_tool asset: {e}"))?
    .ok_or_else(|| "framework_tool asset not found in latest release".to_string())?;
    info!(
        "Attempting direct download of framework_tool into '{}' from '{}'",
        base_dir.to_string_lossy(),
        url
    );
    let final_path = dl::download_to_path(&url, &base_dir.to_string_lossy().to_string()).await?;

    if let Ok(meta) = std::fs::metadata(&final_path) {
        info!("downloaded size: {} bytes", meta.len());
    }
    Ok(())
}
