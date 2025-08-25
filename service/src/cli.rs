use tokio::process::Command;
use tracing::{error, info, warn};
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

// Helper: locate winget.exe for service context (LocalSystem)
#[cfg(target_os = "windows")]
fn find_winget_path() -> Option<String> {
    // Prefer %WINDIR% based System32 path to avoid drive-letter assumptions
    if let Ok(windir) = std::env::var("WINDIR") {
        let sys32 = std::path::Path::new(&windir)
            .join("System32")
            .join("winget.exe");
        if sys32.exists() {
            return sys32.to_str().map(|s| s.to_string());
        }
    }

    // Probe %ProgramFiles%\WindowsApps for packaged DesktopAppInstaller
    let windows_apps_root = std::env::var("ProgramFiles")
        .ok()
        .map(|pf| std::path::Path::new(&pf).join("WindowsApps"))
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Program Files\WindowsApps"));
    if let Ok(entries) = std::fs::read_dir(&windows_apps_root) {
        let mut dirs: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        // Newest first by folder name is a decent heuristic
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
fn find_winget_path() -> Option<String> {
    None
}

/// Try to install framework_tool using winget on Windows.
/// Returns Ok(()) if installation command succeeded (not necessarily that binary is already on PATH).
#[cfg(target_os = "windows")]
pub async fn attempt_install_via_winget() -> Result<(), String> {
    use tokio::time::{timeout, Duration};
    // Build the command string with additional flags suitable for service context
    let winget_path = find_winget_path();
    let winget_invocation = if let Some(p) = &winget_path {
        format!("& '{}'", p.replace("'", "''"))
    } else {
        "winget".to_string()
    };
    // Pick install location relative to the service binary directory; fail if unknown
    let install_location = match std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .and_then(|d| d.to_str().map(|s| s.to_string()))
    {
        Some(s) => s,
        None => return Err("could not resolve service directory for winget install".into()),
    };
    let install_cmd = format!(
        "{} install FrameworkComputer.framework_tool --accept-source-agreements --accept-package-agreements --silent --disable-interactivity --scope machine --location '{}'",
        winget_invocation,
        install_location.replace("'", "''")
    );

    // Log environment context for diagnostics
    let path_env = std::env::var("PATH").unwrap_or_default();
    let cwd = std::env::current_dir()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_default();
    info!(
        "Attempting to install framework_tool via winget (service context). CWD='{}' PATH='{}'",
        cwd, path_env
    );
    if let Some(p) = &winget_path {
        info!("Resolved winget path candidate: {}", p);
    } else {
        warn!("No explicit winget.exe path found; relying on PATH");
    }

    // Log effective user
    if let Ok(out) = Command::new("whoami")
        .stdout(std::process::Stdio::piped())
        .spawn()
    {
        if let Ok(o) = timeout(Duration::from_secs(2), out.wait_with_output()).await {
            if let Ok(o) = o {
                info!("whoami: {}", String::from_utf8_lossy(&o.stdout).trim());
            }
        }
    }

    // Execute winget either directly (preferred) or via PowerShell if not found on PATH
    let child = if let Some(winget_exe) = &winget_path {
        info!(
            "Running direct winget: {} install FrameworkComputer.framework_tool ...",
            winget_exe
        );
        Command::new(winget_exe)
            .args([
                "install",
                "FrameworkComputer.framework_tool",
                "--accept-source-agreements",
                "--accept-package-agreements",
                "--silent",
                "--disable-interactivity",
                "--scope",
                "machine",
                "--location",
                &install_location,
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn winget directly: {e}"))?
    } else {
        // Use powershell to ensure non-interactive winget call works in service context
        let ps_args = [
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &install_cmd,
        ];
        info!("Running PowerShell command: {}", install_cmd);
        Command::new("powershell")
            .args(ps_args)
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
        info!(
            "winget reported successful installation of framework_tool. stdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(
            "winget install exit {}\nstdout: {}\nstderr: {}",
            output.status, stdout, stderr
        );
        Err(format!(
            "winget install failed with status {}",
            output.status
        ))
    }
}

#[cfg(not(target_os = "windows"))]
pub async fn attempt_install_via_winget() -> Result<(), String> {
    Err("winget installation is only supported on Windows".into())
}

/// Resolve framework_tool, attempting installation if not present.
pub async fn resolve_or_install() -> Result<FrameworkTool, String> {
    // 1) Try resolve immediately
    if let Ok(cli) = FrameworkTool::new().await {
        return Ok(cli);
    }

    // 2) Try winget install once
    if let Err(err) = attempt_install_via_winget().await {
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
    let (url, filename) = if cfg!(windows) {
        (
            "https://github.com/FrameworkComputer/framework-system/releases/latest/download/framework_tool.exe",
            "framework_tool.exe",
        )
    } else {
        (
            "https://github.com/FrameworkComputer/framework-system/releases/latest/download/framework_tool",
            "framework_tool",
        )
    };
    let dest_path = base_dir.join(filename).to_string_lossy().to_string();

    info!(
        "Attempting direct download of framework_tool to '{}' from '{}'",
        dest_path, url
    );
    if let Some(parent) = std::path::Path::new(&dest_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Use reqwest to download
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))?;
    let mut resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let mut file = tokio::fs::File::create(&dest_path)
        .await
        .map_err(|e| format!("failed to create dest file: {e}"))?;
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download read failed: {e}"))?
    {
        use tokio::io::AsyncWriteExt;
        file.write_all(&chunk)
            .await
            .map_err(|e| format!("write failed: {e}"))?;
    }
    {
        use tokio::io::AsyncWriteExt;
        file.flush()
            .await
            .map_err(|e| format!("flush failed: {e}"))?;
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&dest_path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&dest_path, perms);
        }
    }

    if let Ok(meta) = std::fs::metadata(&dest_path) {
        info!("downloaded size: {} bytes", meta.len());
    }
    Ok(())
}
