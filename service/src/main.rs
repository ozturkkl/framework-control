use std::net::SocketAddr;
use std::fs::OpenOptions;
use std::io::Write;

use axum::extract::State;
use axum::http::Method;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use which::which;
use tokio::process::Command;

#[derive(Clone)]
struct AppState {}

#[derive(Serialize)]
struct PowerResponse {
    ac_present: bool,
    battery: Option<BatteryReduced>,
}

#[derive(Serialize)]
struct BatteryReduced {
    cycle_count: u32,
    charge_percentage: u32,
    charging: bool,
}

#[derive(Deserialize)]
struct FanDutyRequest {
    fan_index: Option<u32>,
    percent: u32,
}

#[tokio::main]
async fn main() {
    // Write startup log
    let log_path = r"C:\Program Files\FrameworkControl\service.log";
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap_or_else(|_| {
            // If can't write to Program Files, try temp
            let temp_path = std::env::temp_dir().join("framework-control.log");
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(temp_path)
                .expect("Failed to open log file")
        });
    
    writeln!(log_file, "\n=== Service starting at {} ===", chrono::Local::now()).ok();
    writeln!(log_file, "Current user: {:?}", std::env::var("USERNAME")).ok();
    writeln!(log_file, "PATH: {:?}", std::env::var("PATH")).ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .without_time()
        .init();

    let state = AppState {};

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/power", get(get_power))
        .route("/api/fan/duty", post(set_fan_duty))
        .route("/api/health", get(|| async { "ok" }))
        .with_state(state)
        .layer(cors);

    let host = std::env::var("FRAMEWORK_CONTROL_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8090);
    let addr: SocketAddr = (host.parse::<std::net::IpAddr>().unwrap(), port).into();
    info!("listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_power(_state: State<AppState>) -> Json<PowerResponse> {
    // Use CLI: framework_tool --power (parse minimal fields)
    let cli = resolve_framework_tool().await;
    match cli {
        Ok(path) => {
            info!("Using framework_tool at: {}", path);
            match run_and_capture(&path, &["--power"]).await {
                Ok(output) => {
                    info!("Got output ({} bytes): {}", output.len(), 
                        if output.len() > 100 { &output[..100] } else { &output });
                    Json(parse_power(&output))
                },
                Err(e) => {
                    error!("power exec error: {}", e);
                    Json(PowerResponse { ac_present: false, battery: None })
                }
            }
        },
        Err(e) => {
            error!("Failed to find framework_tool: {}", e);
            Json(PowerResponse { ac_present: false, battery: None })
        },
    }
}

async fn set_fan_duty(
    _state: State<AppState>,
    Json(req): Json<FanDutyRequest>,
) -> Json<serde_json::Value> {
    // Use CLI: framework_tool --fansetduty [fan percent]
    // Support optional fan index by passing two args when provided
    let cli = resolve_framework_tool().await;
    match cli {
        Ok(path) => {
            let percent_s = req.percent.to_string();
            let fan_idx_s = req.fan_index.map(|idx| idx.to_string());
            let mut args: Vec<&str> = vec!["--fansetduty"]; 
            if let Some(ref idxs) = fan_idx_s { args.push(idxs.as_str()); }
            args.push(percent_s.as_str());
            match run_and_capture(&path, &args).await {
                Ok(_) => Json(serde_json::json!({ "status": "ok" })),
                Err(e) => Json(serde_json::json!({ "status": "error", "error": e })),
            }
        }
        Err(e) => Json(serde_json::json!({ "status": "error", "error": e })),
    }
}

async fn resolve_framework_tool() -> Result<String, String> {
    // Log to file for debugging
    let log_path = r"C:\Program Files\FrameworkControl\service.log";
    if let Ok(mut f) = OpenOptions::new().append(true).open(log_path) {
        writeln!(f, "Searching for framework_tool...").ok();
        writeln!(f, "PATH: {:?}", std::env::var("PATH")).ok();
    }
    // Allow override via env var
    if let Ok(p) = std::env::var("FRAMEWORK_TOOL_PATH") {
        let path = std::path::Path::new(&p);
        if path.exists() {
            return Ok(p);
        }
    }
    
    // Try common locations first
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
    
    // Try which
    if let Ok(p) = which("framework_tool") { return Ok(p.to_string_lossy().to_string()); }
    if let Ok(p) = which("framework_tool.exe") { return Ok(p.to_string_lossy().to_string()); }
    
    Err("framework_tool not found. Please install via winget: winget install FrameworkComputer.framework_tool".into())
}

async fn run_and_capture(bin: &str, args: &[&str]) -> Result<String, String> {
    let out = Command::new(bin)
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

fn parse_power(text: &str) -> PowerResponse {
    // Parse minimal fields from framework_tool --power output
    let mut ac_present = false;
    let mut charge_percentage: Option<u32> = None;
    let mut cycle_count: Option<u32> = None;
    let mut charging = false;

    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("AC is:") {
            ac_present = l.to_ascii_lowercase().contains("connected");
        } else if l.starts_with("Battery SoC:") || l.starts_with("Charge level:") {
            // e.g. "Battery SoC: 90%" or "Charge level: 89%"
            if let Some(val) = l.split(':').nth(1) { 
                if let Some(num) = val.trim().trim_end_matches('%').split_whitespace().next() { 
                    charge_percentage = num.parse().ok(); 
                } 
            }
        } else if l.starts_with("Cycle Count:") {
            if let Some(val) = l.split(':').nth(1) { 
                cycle_count = val.trim().parse().ok(); 
            }
        } else if l.contains("Battery charging") || l.contains("Battery is charging") { 
            charging = true; 
        }
    }

    PowerResponse {
        ac_present,
        battery: if charge_percentage.is_some() || cycle_count.is_some() {
            Some(BatteryReduced { 
                cycle_count: cycle_count.unwrap_or(0), 
                charge_percentage: charge_percentage.unwrap_or(0), 
                charging 
            })
        } else { None }
    }
}


