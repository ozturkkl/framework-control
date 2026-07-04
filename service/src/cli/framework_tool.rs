use super::framework_tool_parser::{
    parse_power, parse_thermal, parse_versions, PowerBatteryInfo, ThermalParsed, VersionsParsed,
};
use crate::types::FrameworkToolConfig;
use crate::utils::{download as dl, github as gh, global_cache};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// True after a framework_tool process fails (spawn/exit/timeout), cleared on
/// success, so the resolver re-validates only when a real call hit trouble.
static TOOL_SUSPECT: AtomicBool = AtomicBool::new(false);

const TOOL_REPO: (&str, &str) = ("FrameworkComputer", "framework-system");
const TOOL_ASSET: &str = if cfg!(windows) {
    "framework_tool.exe"
} else {
    "framework_tool"
};
pub fn tool_suspect() -> bool {
    TOOL_SUSPECT.load(Ordering::Relaxed)
}

#[cfg(target_os = "windows")]
use crate::utils::wget as wg;
use tokio::process::Command;
use tracing::{error, info, warn};
use which::which;

/// Thin wrapper around the `framework_tool` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
///
/// Resolution strategy:
/// - Prefer a copy alongside the running service binary (direct downloads)
/// - Then fall back to `framework_tool` on `PATH`
/// - Windows can optionally auto-install via winget;
#[derive(Clone)]
pub struct FrameworkTool {
    pub(crate) path: String,
}

impl FrameworkTool {
    pub async fn at_path(path: String) -> Result<Self, String> {
        info!("framework_tool resolved at: {}", path);
        let cli = Self { path };
        // Validate the binary is runnable with a lightweight call.
        if let Err(e) = cli.versions().await {
            return Err(format!("framework_tool not runnable: {}", e));
        }
        Ok(cli)
    }

    pub async fn power(&self) -> Result<PowerBatteryInfo, String> {
        const TTL: Duration = Duration::from_secs(5);
        global_cache::cache_get_or_update("framework_tool.power", TTL, true, || async {
            let out = self.run(&["--power", "-vv"]).await?;
            Ok(parse_power(&out))
        })
        .await
    }

    pub async fn thermal(&self) -> Result<ThermalParsed, String> {
        const TTL: Duration = Duration::from_secs(1);
        global_cache::cache_get_or_update("framework_tool.thermal", TTL, true, || async {
            let out = self.run(&["--thermal"]).await?;
            Ok(parse_thermal(&out))
        })
        .await
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

    /// Get charge limit min/max percentage as reported by EC
    pub async fn charge_limit_get(&self) -> Result<super::framework_tool_parser::BatteryChargeLimitInfo, String> {
        use super::framework_tool_parser::parse_charge_limit;
        const TTL: Duration = Duration::from_secs(5);
        global_cache::cache_get_or_update("framework_tool.charge_limit", TTL, true, || async {
            let out = self.run(&["--charge-limit"]).await?;
            let info = parse_charge_limit(&out);
            if info.charge_limit_min_pct.is_some() || info.charge_limit_max_pct.is_some() {
                Ok(info)
            } else {
                Err("failed to parse charge limit".to_string())
            }
        })
        .await
    }

    /// Set max charge limit percentage
    pub async fn charge_limit_set(&self, max_pct: u8) -> Result<(), String> {
        let arg = max_pct.to_string();
        let _ = self.run(&["--charge-limit", &arg]).await?;
        Ok(())
    }

    /// Set charge rate limit in C; optional SoC threshold in percent
    pub async fn charge_rate_limit_set(&self, rate_c: f32, soc_threshold_pct: Option<u8>) -> Result<(), String> {
        let rate = format!("{:.3}", rate_c);
        match soc_threshold_pct {
            Some(soc) => {
                let s = soc.to_string();
                let _ = self.run(&["--charge-rate-limit", &rate, &s]).await?;
            }
            None => {
                let _ = self.run(&["--charge-rate-limit", &rate]).await?;
            }
        }
        Ok(())
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        use tokio::time::{timeout, Duration};
        let result = async {
            let child = Command::new(&self.path)
                .args(args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("spawn failed: {e}"))?;
            let output = timeout(Duration::from_secs(60), child.wait_with_output())
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
        .await;
        TOOL_SUSPECT.store(result.is_err(), Ordering::Relaxed);
        result
    }
}

fn install_path() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    Some(dir.join(TOOL_ASSET))
}

const LATEST_INSTALL_FAILED_FLAG: &str = ".framework_tool_latest_install_failed";

fn latest_install_failed_flag() -> Option<std::path::PathBuf> {
    install_path()?.parent().map(|d| d.join(LATEST_INSTALL_FAILED_FLAG))
}

fn is_latest_install_failed() -> bool {
    latest_install_failed_flag().is_some_and(|p| p.is_file())
}

fn set_latest_install_failed() {
    if let Some(path) = latest_install_failed_flag() {
        let _ = std::fs::write(path, b"");
    }
}

pub fn clear_latest_install_failed() {
    if let Some(path) = latest_install_failed_flag() {
        let _ = std::fs::remove_file(path);
    }
}

/// Recent framework_tool release tags with a downloadable binary for this
/// platform, newest first (cached to spare GitHub rate limits)
pub async fn list_available_versions() -> Result<Vec<String>, String> {
    const TTL: Duration = Duration::from_secs(600);
    global_cache::cache_get_or_update("framework_tool.release_tags", TTL, true, || async {
        gh::list_release_tags(TOOL_REPO.0, TOOL_REPO.1, 10, TOOL_ASSET).await
    })
    .await
}

pub async fn latest_tag() -> Result<String, String> {
    list_available_versions()
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| "no framework_tool releases found".to_string())
}

/// All places a framework_tool binary may already exist: install path first, then PATH.
fn candidate_paths() -> Vec<String> {
    let mut paths = Vec::new();
    if let Some(m) = install_path() {
        if m.exists() {
            paths.push(m.to_string_lossy().to_string());
        }
    }
    if let Ok(p) = which(TOOL_ASSET) {
        let s = p.to_string_lossy().to_string();
        if !paths.contains(&s) {
            paths.push(s);
        }
    }
    paths
}

/// Find any runnable copy (install path, then PATH; winget once on Windows).
async fn resolve_existing() -> Result<FrameworkTool, String> {
    for path in candidate_paths() {
        if let Ok(cli) = FrameworkTool::at_path(path).await {
            return Ok(cli);
        }
    }
    #[cfg(windows)]
    {
        if let Err(err) = wg::try_winget_install_package("FrameworkComputer.framework_tool", None).await {
            warn!("winget automatic install failed: {}", err);
        }
        for path in candidate_paths() {
            if let Ok(cli) = FrameworkTool::at_path(path).await {
                return Ok(cli);
            }
        }
    }
    error!("framework_tool not found or not runnable after attempted installs");
    Err("framework_tool not found or not runnable".into())
}

/// Ensure the install-path copy matches `tag` (reuse if already there, else download).
/// On failure the on-disk install copy is left unchanged.
pub async fn install_version(tag: &str) -> Result<(), String> {
    let want = tag.trim_start_matches('v');

    if let Some(installed) = install_path() {
        if installed.exists() {
            let cli = FrameworkTool {
                path: installed.to_string_lossy().to_string(),
            };
            if cli.versions().await.ok().and_then(|v| v.tool_version).as_deref() == Some(want) {
                info!("framework_tool {} already at {}", tag, installed.display());
                return Ok(());
            }
        }
    }

    attempt_install_via_direct_download(tag)
        .await
        .map_err(|e| format!("framework_tool {} download failed: {}", tag, e))?;

    let installed = install_path().ok_or_else(|| "could not resolve service directory".to_string())?;
    match FrameworkTool::at_path(installed.to_string_lossy().to_string()).await {
        Ok(_) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&installed);
            Err(format!("downloaded framework_tool {} is not runnable here: {}", tag, e))
        }
    }
}

/// Resolve framework_tool per config: keep it on the latest release when opted
/// in, otherwise use any available copy (installing the latest release only
/// when none exists at all).
pub async fn resolve_or_install(cfg: &FrameworkToolConfig) -> Result<FrameworkTool, String> {
    if !cfg.latest {
        if let Ok(cli) = resolve_existing().await {
            return Ok(cli);
        }
        // Nothing runnable anywhere — bootstrap from latest.
        if let Ok(tag) = latest_tag().await {
            let _ = install_version(&tag).await;
        }
        return resolve_existing().await;
    }

    if !is_latest_install_failed() {
        match latest_tag().await {
            Ok(tag) => {
                if let Err(e) = install_version(&tag).await {
                    warn!("framework_tool {} unavailable ({}); using any available copy", tag, e);
                    set_latest_install_failed();
                }
            }
            Err(e) => warn!(
                "could not determine latest framework_tool version ({}); using any available copy",
                e
            ),
        }
    }
    resolve_existing().await
}

/// Cross-platform direct download of framework_tool at the given release tag
async fn attempt_install_via_direct_download(tag: &str) -> Result<(), String> {
    // Always download next to the service binary to avoid hardcoded system paths
    let base_dir = match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
    {
        Some(p) => p,
        None => return Err("could not resolve service directory for direct download".into()),
    };
    let url = gh::get_release(TOOL_REPO.0, TOOL_REPO.1, Some(tag), &[TOOL_ASSET])
        .await
        .map_err(|e| format!("failed to resolve framework_tool asset: {e}"))?
        .ok_or_else(|| "framework_tool asset not found in release".to_string())?;
    info!(
        "Attempting direct download of framework_tool into '{}' from '{}'",
        base_dir.to_string_lossy(),
        url
    );
    let final_path = dl::download_to_path(&url, &base_dir.to_string_lossy().to_string()).await?;

    if let Ok(meta) = std::fs::metadata(&final_path) {
        info!("downloaded size: {} bytes", meta.len());
    }

    // Make executable on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&final_path)
            .map_err(|e| format!("failed to get binary metadata: {}", e))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&final_path, perms)
            .map_err(|e| format!("failed to set executable permissions: {}", e))?;
        info!("set executable permissions on framework_tool");
    }

    Ok(())
}
