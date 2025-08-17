use std::fs::OpenOptions;
use std::io::Write;

use tokio::process::Command;
use tracing::{info};
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
        let out = Command::new(&self.path)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn failed: {e}"))?
            .wait_with_output().await
            .map_err(|e| format!("wait failed: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(format!("exit {}: {}", out.status, String::from_utf8_lossy(&out.stderr)))
        }
    }
}

async fn resolve_framework_tool() -> Result<String, String> {
    let log_path = r"C:\Program Files\FrameworkControl\service.log";
    if let Ok(mut f) = OpenOptions::new().append(true).open(log_path) {
        writeln!(f, "Searching for framework_tool...").ok();
        writeln!(f, "PATH: {:?}", std::env::var("PATH")).ok();
    }

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
            if let Ok(mut f) = OpenOptions::new().append(true).open(log_path) {
                writeln!(f, "Found at: {}", path).ok();
            }
            return Ok(path.to_string());
        }
    }

    if let Ok(p) = which("framework_tool") { return Ok(p.to_string_lossy().to_string()); }
    if let Ok(p) = which("framework_tool.exe") { return Ok(p.to_string_lossy().to_string()); }

    Err("framework_tool not found. Please install via winget: winget install FrameworkComputer.framework_tool".into())
}


