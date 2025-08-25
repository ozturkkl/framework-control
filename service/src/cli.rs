
use tokio::process::Command;
use tracing::{info, warn, error};
use which::which;

/// Thin wrapper around the `framework_tool` CLI.
/// Resolves the binary path once and provides async helpers to run commands.
#[derive(Clone)]
pub struct FrameworkTool {
    path: String,
}

impl FrameworkTool {
    pub async fn new() -> Result<Self, String> {
        let path = resolve_framework_tool().await?;
        info!("framework_tool resolved at: {}", path);
        Ok(Self { path })
    }

    pub async fn power(&self) -> Result<String, String> {
        self.run(&["--power"]).await
    }

    pub async fn thermal(&self) -> Result<String, String> {
        self.run(&["--thermal"]).await
    }

    pub async fn versions(&self) -> Result<String, String> {
        self.run(&["--versions"]).await
    }

    pub async fn set_fan_duty(&self, percent: u32, fan_index: Option<u32>) -> Result<(), String> {
        let percent_s = percent.to_string();
        let fan_idx_s = fan_index.map(|idx| idx.to_string());
        let mut args: Vec<&str> = vec!["--fansetduty"]; 
        if let Some(ref idxs) = fan_idx_s { args.push(idxs.as_str()); }
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
            Err(format!("exit {}: {}", output.status, String::from_utf8_lossy(&output.stderr)))
        }
    }
}

async fn resolve_framework_tool() -> Result<String, String> {

    if let Ok(p) = std::env::var("FRAMEWORK_TOOL_PATH") {
        let path = std::path::Path::new(&p);
        if path.exists() {
            return Ok(p);
        }
    }

    let common_paths = [
        r"C:\Program Files\FrameworkControl\framework_tool.exe",
        r"C:\Users\Kemal\AppData\Local\Microsoft\WinGet\Links\framework_tool.exe",
        r"C:\Program Files\WindowsApps\FrameworkComputer.framework_tool_0.1.0.0_x64__gzpqkc1j3p5n0\framework_tool.exe",
        r"C:\Windows\System32\framework_tool.exe",
    ];

    for path in &common_paths {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    if let Ok(p) = which("framework_tool") { return Ok(p.to_string_lossy().to_string()); }
    if let Ok(p) = which("framework_tool.exe") { return Ok(p.to_string_lossy().to_string()); }

    Err("framework_tool not found. Please install via winget: winget install FrameworkComputer.framework_tool".into())
}

/// Try to install framework_tool using winget on Windows.
/// Returns Ok(()) if installation command succeeded (not necessarily that binary is already on PATH).
#[cfg(target_os = "windows")]
pub async fn attempt_install_via_winget() -> Result<(), String> {
    use tokio::time::{timeout, Duration};
    // Use powershell to ensure non-interactive winget call works in service context
    let args = [
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        // Accept agreements and try to be quiet; winget may still emit output
        "winget install FrameworkComputer.framework_tool --accept-source-agreements --accept-package-agreements --silent",
    ];
    info!("Attempting to install framework_tool via winget...");
    let child = Command::new("powershell")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn winget: {e}"))?;
    let output = timeout(Duration::from_secs(180), child.wait_with_output())
        .await
        .map_err(|_| "winget install timed out".to_string())
        .and_then(|r| r.map_err(|e| format!("winget wait failed: {e}")))?;
    if output.status.success() {
        info!("winget reported successful installation of framework_tool");
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("winget install exit {}: {}", output.status, stderr);
        Err(format!("winget install failed with status {}", output.status))
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn attempt_install_via_winget() -> Result<(), String> {
    Err("winget installation is only supported on Windows".into())
}

/// Resolve framework_tool, attempting installation if not present.
pub async fn resolve_or_install() -> Result<FrameworkTool, String> {
    match FrameworkTool::new().await {
        Ok(cli) => Ok(cli),
        Err(e) => {
            warn!("framework_tool not found: {}", e);
            match attempt_install_via_winget().await {
                Ok(()) => {
                    // Try resolve again
                    match FrameworkTool::new().await {
                        Ok(cli) => Ok(cli),
                        Err(e2) => {
                            error!("framework_tool still not found after install: {}", e2);
                            Err(e2)
                        }
                    }
                }
                Err(ei) => {
                    warn!("automatic install failed: {}", ei);
                    Err(e)
                }
            }
        }
    }
}


