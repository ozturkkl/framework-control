use std::pin::Pin;
use std::time::Duration;

use crate::shortcuts;
use crate::state::AppState;
use crate::types::{Config, ConfigEvent, Empty, Health, PartialConfig, ShortcutsStatus, SystemInfo, UpdateCheck};
use crate::update::{check_and_apply_now, get_current_and_latest};
use poem::web::Data;
use poem_openapi::param::Query;
use poem_openapi::payload::{EventStream, Json};
use poem_openapi::{ApiResponse, OpenApi};
use sysinfo::System;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};
use tracing::{error, info};

#[derive(ApiResponse)]
enum ApiErrorResponse {
    #[oai(status = 502)]
    BadGateway(Json<crate::types::ErrorEnvelope>),
    #[oai(status = 503)]
    ServiceUnavailable(Json<crate::types::ErrorEnvelope>),
}

type ApiResult<T> = Result<Json<T>, ApiErrorResponse>;

async fn require_framework_tool_async(
    state: &AppState,
) -> Result<crate::cli::framework_tool::FrameworkTool, ApiErrorResponse> {
    let cli_opt = { state.framework_tool.read().await.clone() };
    match cli_opt {
        Some(cli) => Ok(cli),
        None => Err(ApiErrorResponse::ServiceUnavailable(Json(
            crate::types::ErrorEnvelope {
                code: "cli_unavailable".into(),
                message: "framework_tool not found".into(),
            },
        ))),
    }
}

#[cfg(target_os = "windows")]
async fn require_ryzenadj_async(state: &AppState) -> Result<crate::cli::ryzen_adj::RyzenAdj, ApiErrorResponse> {
    let cli_opt = { state.ryzenadj.read().await.clone() };
    match cli_opt {
        Some(cli) => Ok(cli),
        None => Err(ApiErrorResponse::ServiceUnavailable(Json(
            crate::types::ErrorEnvelope {
                code: "ryzenadj_unavailable".into(),
                message: "ryzenadj not found".into(),
            },
        ))),
    }
}

#[cfg(target_os = "linux")]
async fn require_linux_power_async(state: &AppState) -> Result<crate::cli::linux_power::LinuxPower, ApiErrorResponse> {
    let cli_opt = { state.linux_power.read().await.clone() };
    match cli_opt {
        Some(cli) => Ok(cli),
        None => Err(ApiErrorResponse::ServiceUnavailable(Json(
            crate::types::ErrorEnvelope {
                code: "linux_power_unavailable".into(),
                message: "linux power management not available".into(),
            },
        ))),
    }
}

fn bad_gateway(code: &str, message: String) -> ApiErrorResponse {
    ApiErrorResponse::BadGateway(Json(crate::types::ErrorEnvelope {
        code: code.into(),
        message,
    }))
}

fn map_cli_err(e: String) -> ApiErrorResponse {
    bad_gateway("cli_failed", e)
}

pub struct Api;

#[OpenApi]
impl Api {
    /// Health: returns overall service health and CLI presence
    #[oai(path = "/health", method = "get", operation_id = "health")]
    async fn health(&self, state: Data<&AppState>) -> ApiResult<Health> {
        let cli_present = state.framework_tool.read().await.is_some();
        let service_version = env!("CARGO_PKG_VERSION").to_string();
        Ok(Json(Health {
            cli_present,
            service_version,
        }))
    }

    /// RyzenAdj: install on demand (Windows only)
    #[oai(path = "/ryzenadj/install", method = "post", operation_id = "installRyzenadj")]
    async fn install_ryzenadj(&self) -> ApiResult<Empty> {
        #[cfg(target_os = "windows")]
        {
            match crate::cli::ryzen_adj::attempt_install_via_direct_download().await {
                Ok(_) => {
                    // Validate resolve, but do not spawn another task (boot task will pick it up)
                    match crate::cli::ryzen_adj::RyzenAdj::new().await {
                        Ok(_cli) => Ok(Json(Empty {})),
                        Err(e) => {
                            error!("ryzenadj resolve after install failed: {}", e);
                            Err(bad_gateway("ryzenadj_unavailable", e))
                        }
                    }
                }
                Err(e) => Err(bad_gateway("install_failed", e)),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(bad_gateway(
                "unsupported_platform",
                "RyzenAdj is only available on Windows. Linux uses native kernel interfaces.".to_string(),
            ))
        }
    }

    #[oai(path = "/ryzenadj/uninstall", method = "post", operation_id = "uninstallRyzenadj")]
    async fn uninstall_ryzenadj(&self, _state: Data<&AppState>) -> ApiResult<Empty> {
        #[cfg(target_os = "windows")]
        {
            match crate::cli::ryzen_adj::remove_installed_files().await {
                Ok(_) => {
                    {
                        let mut w = _state.ryzenadj.write().await;
                        *w = None;
                    }
                    Ok(Json(Empty {}))
                }
                Err(e) => {
                    error!("uninstall ryzenadj failed: {}", e);
                    Err(bad_gateway("uninstall_failed", e))
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Err(bad_gateway(
                "unsupported_platform",
                "RyzenAdj is only available on Windows. Linux uses native kernel interfaces.".to_string(),
            ))
        }
    }

    #[oai(path = "/power", method = "get", operation_id = "getPower")]
    async fn get_power(&self, state: Data<&AppState>) -> ApiResult<crate::types::PowerResponse> {
        let cli = require_framework_tool_async(&state).await?;
        let p = cli.power().await.map_err(map_cli_err)?;

        let limits = match cli.charge_limit_get().await {
            Ok(info) => info,
            Err(_e) => Default::default(),
        };
        let battery_api: Option<crate::types::BatteryInfo> = Some(crate::types::BatteryInfo {
            power_info: p.clone(),
            limits,
        });

        let power_control = {
            #[cfg(target_os = "windows")]
            {
                if let Ok(ryz) = require_ryzenadj_async(&state).await {
                    let capabilities = ryz.get_capabilities();
                    let current_state = ryz.get_state().await.unwrap_or_default();
                    crate::types::PowerControlInfo {
                        capabilities,
                        current_state,
                    }
                } else {
                    crate::types::PowerControlInfo {
                        capabilities: Default::default(),
                        current_state: Default::default(),
                    }
                }
            }

            #[cfg(target_os = "linux")]
            {
                if let Ok(lp) = require_linux_power_async(&state).await {
                    let capabilities = lp.get_capabilities().await;
                    let current_state = lp.get_state().await.unwrap_or_default();
                    crate::types::PowerControlInfo {
                        capabilities,
                        current_state,
                    }
                } else {
                    crate::types::PowerControlInfo {
                        capabilities: Default::default(),
                        current_state: Default::default(),
                    }
                }
            }

            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            {
                crate::types::PowerControlInfo {
                    capabilities: Default::default(),
                    current_state: Default::default(),
                }
            }
        };

        Ok(Json(crate::types::PowerResponse {
            battery: battery_api,
            power_control,
        }))
    }

    #[oai(path = "/update/check", method = "get", operation_id = "checkUpdate")]
    async fn check_update(&self) -> ApiResult<UpdateCheck> {
        if !crate::update::updates_enabled() {
            info!("FRAMEWORK_CONTROL_UPDATE_REPO not set; in-app updates disabled");
            let current = env!("CARGO_PKG_VERSION").trim().to_string();
            return Ok(Json(UpdateCheck {
                current_version: current.clone(),
                latest_version: current,
                updates_enabled: false,
            }));
        }
        match get_current_and_latest().await {
            Ok((current, latest)) => Ok(Json(UpdateCheck {
                current_version: current,
                latest_version: latest,
                updates_enabled: true,
            })),
            Err(e) => {
                error!("update check failed: {}", e);
                Err(bad_gateway("update_check_failed", e))
            }
        }
    }

    #[oai(path = "/update/apply", method = "post", operation_id = "applyUpdate")]
    async fn apply_update(&self) -> ApiResult<Empty> {
        match check_and_apply_now().await {
            Ok(_applied) => Ok(Json(Empty {})),
            Err(e) => {
                error!("apply update failed: {}", e);
                Err(bad_gateway("apply_failed", e))
            }
        }
    }

    #[oai(path = "/thermal", method = "get", operation_id = "getThermal")]
    async fn get_thermal(&self, state: Data<&AppState>) -> ApiResult<crate::cli::framework_tool_parser::ThermalParsed> {
        let cli = require_framework_tool_async(&state).await?;
        let v = cli.thermal().await.map_err(map_cli_err)?;
        Ok(Json(v))
    }

    #[oai(path = "/thermal/history", method = "get", operation_id = "getThermalHistory")]
    async fn get_thermal_history(&self, state: Data<&AppState>) -> ApiResult<Vec<crate::types::TelemetrySample>> {
        let samples: Vec<crate::types::TelemetrySample> = {
            let r = state.telemetry_samples.read().await;
            r.iter().cloned().collect()
        };
        Ok(Json(samples))
    }

    #[oai(path = "/versions", method = "get", operation_id = "getVersions")]
    async fn get_versions(
        &self,
        state: Data<&AppState>,
    ) -> ApiResult<crate::cli::framework_tool_parser::VersionsParsed> {
        let cli = require_framework_tool_async(&state).await?;
        let v = cli.versions().await.map_err(map_cli_err)?;
        Ok(Json(v))
    }

    #[oai(path = "/config", method = "get", operation_id = "getConfig")]
    async fn get_config(&self, state: Data<&AppState>) -> ApiResult<crate::types::Config> {
        let cfg = state.config.read().await.clone();
        Ok(Json(cfg))
    }

    /// Config write events to keep clients in sync
    #[oai(path = "/config/events", method = "get", operation_id = "getConfigEvents")]
    async fn get_config_events(
        &self,
        state: Data<&AppState>,
    ) -> EventStream<Pin<Box<dyn Stream<Item = ConfigEvent> + Send>>> {
        let rx = state.config.subscribe();
        let stream: Pin<Box<dyn Stream<Item = ConfigEvent> + Send>> = Box::pin(
            BroadcastStream::new(rx)
                .take_while(|r| r.is_ok())
                .filter_map(|r| r.ok()),
        );
        EventStream::new(stream).keep_alive(Duration::from_secs(15))
    }

    #[oai(path = "/config", method = "post", operation_id = "setConfig")]
    async fn set_config(
        &self,
        state: Data<&AppState>,
        #[oai(name = "client_id", default)] client_id: Query<Option<String>>,
        req: Json<PartialConfig>,
    ) -> ApiResult<Config> {
        let req = req.0;
        let client_id = client_id.0.filter(|s| !s.is_empty());
        match state.config.persist_partial(client_id, req).await {
            Ok(cfg) => {
                info!("set_config applied successfully");
                Ok(Json(cfg))
            }
            Err(e) => {
                error!("config save error: {}", e);
                Err(bad_gateway("save_failed", e))
            }
        }
    }

    #[oai(
        path = "/framework_tool/versions",
        method = "get",
        operation_id = "getFrameworkToolVersions"
    )]
    async fn get_framework_tool_versions(
        &self,
        state: Data<&AppState>,
    ) -> ApiResult<crate::types::FrameworkToolVersions> {
        use crate::cli::framework_tool::{latest_tag, list_available_versions};
        let mut out = crate::types::FrameworkToolVersions::default();
        if let Some(cli) = state.framework_tool.read().await.clone() {
            out.current_version = cli.versions().await.ok().and_then(|v| v.tool_version);
        }
        out.available_tags = list_available_versions().await.unwrap_or_default();
        out.latest_tag = latest_tag().await.ok();
        Ok(Json(out))
    }

    async fn set_tool_latest(state: &AppState, latest: bool) -> Result<(), ApiErrorResponse> {
        if state.config.read().await.framework_tool.latest == latest {
            return Ok(());
        }
        if let Err(e) = state
            .config
            .persist(None, |cfg| cfg.framework_tool.latest = latest)
            .await
        {
            error!("config save error: {}", e);
            return Err(bad_gateway("save_failed", e));
        }
        Ok(())
    }

    #[oai(
        path = "/framework_tool/switch",
        method = "post",
        operation_id = "switchFrameworkToolVersion"
    )]
    async fn switch_framework_tool_version(
        &self,
        state: Data<&AppState>,
        req: Json<crate::types::SwitchFrameworkToolRequest>,
    ) -> ApiResult<Empty> {
        use crate::cli::framework_tool::{clear_latest_install_failed, install_version, latest_tag};
        clear_latest_install_failed();
        match req.0.version {
            Some(tag) => {
                Self::set_tool_latest(&state, false).await?;
                install_version(&tag).await.map_err(|e| {
                    error!("framework_tool install failed: {}", e);
                    bad_gateway("install_failed", "Install failed; check the service logs.".into())
                })?;
            }
            None => {
                Self::set_tool_latest(&state, true).await?;
                let tag = latest_tag().await.map_err(|e| {
                    error!("framework_tool latest tag lookup failed: {}", e);
                    bad_gateway("install_failed", "Install failed; check the service logs.".into())
                })?;
                install_version(&tag).await.map_err(|e| {
                    error!("framework_tool install failed: {}", e);
                    bad_gateway("install_failed", "Install failed; check the service logs.".into())
                })?;
            }
        }
        *state.framework_tool.write().await = None;
        Ok(Json(Empty {}))
    }

    #[oai(path = "/system", method = "get", operation_id = "getSystemInfo")]
    async fn get_system_info(&self) -> ApiResult<SystemInfo> {
        let sys = System::new_all();
        let mut cpu = sys.global_cpu_info().brand().trim().to_string();
        if cpu.is_empty() {
            if let Some(c) = sys.cpus().iter().find(|c| !c.brand().trim().is_empty()) {
                cpu = c.brand().trim().to_string();
            }
        }
        let mem_mb = sys.total_memory() / 1024 / 1024;
        let os = System::name().unwrap_or_else(|| "Unknown OS".into());
        let dgpu = pick_dedicated_gpu(&get_gpu_names().await);
        Ok(Json(SystemInfo {
            cpu,
            memory_total_mb: mem_mb,
            os,
            dgpu,
        }))
    }

    #[oai(path = "/shortcuts/status", method = "get", operation_id = "getShortcutsStatus")]
    async fn get_shortcuts_status(&self) -> ApiResult<ShortcutsStatus> {
        let installed = shortcuts::shortcuts_exist();
        Ok(Json(ShortcutsStatus { installed }))
    }

    #[oai(path = "/shortcuts/create", method = "post", operation_id = "createShortcuts")]
    async fn create_shortcuts(&self) -> ApiResult<Empty> {
        let port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
            .ok()
            .or_else(|| option_env!("FRAMEWORK_CONTROL_PORT").map(String::from))
            .expect("FRAMEWORK_CONTROL_PORT must be set (either at runtime or baked at compile-time)")
            .parse()
            .expect("FRAMEWORK_CONTROL_PORT must be valid");

        match shortcuts::create_shortcuts(port).await {
            Ok(_) => {
                info!("Shortcuts created successfully");
                Ok(Json(Empty {}))
            }
            Err(e) => {
                error!("Failed to create shortcuts: {}", e);
                Err(bad_gateway("shortcuts_failed", e))
            }
        }
    }

    #[oai(path = "/logs", method = "get", operation_id = "getLogs")]
    async fn get_logs(&self) -> Result<poem_openapi::payload::PlainText<String>, ApiErrorResponse> {
        match get_service_logs().await {
            Ok(logs) => Ok(poem_openapi::payload::PlainText(logs)),
            Err(e) => {
                error!("Failed to retrieve logs: {}", e);
                Err(bad_gateway("logs_failed", e))
            }
        }
    }
}

async fn get_service_logs() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        use tokio::process::Command;
        let output = Command::new("journalctl")
            .arg("-u")
            .arg("framework-control")
            .arg("-n")
            .arg("500")
            .arg("--no-pager")
            .output()
            .await
            .map_err(|e| format!("failed to run journalctl: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "journalctl failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().map_err(|e| format!("failed to get current exe path: {}", e))?;
        let dir = exe.parent().ok_or_else(|| "failed to get exe directory".to_string())?;
        let log_path = dir.join("FrameworkControlService.out.log");

        let contents = std::fs::read_to_string(&log_path).map_err(|e| format!("failed to read log file: {}", e))?;

        let lines: Vec<&str> = contents.lines().collect();
        let start = lines.len().saturating_sub(500);
        Ok(lines[start..].join("\n"))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        Err("log retrieval not supported on this platform".to_string())
    }
}

async fn get_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use tokio::process::Command;
        let ps = "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";
        if let Ok(out) = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(ps)
            .output()
            .await
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                return s
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
        }
    }
    Vec::new()
}

fn pick_dedicated_gpu(names: &[String]) -> Option<String> {
    let mut best: Option<String> = None;
    for n in names {
        let lo = n.to_ascii_lowercase();
        let looks_discrete = lo.contains("rtx")
            || lo.contains("gtx")
            || lo.contains("rx ")
            || lo.contains("arc ")
            || lo.contains("radeon pro")
            || lo.contains("geforce")
            || lo.contains("quadro")
            || lo.contains("radeon rx");
        let looks_integrated = lo.contains("uhd") || lo.contains("iris") || lo.contains("vega") || lo.contains("780m");
        if looks_discrete && !looks_integrated {
            return Some(n.clone());
        }
        if best.is_none() && !looks_integrated {
            best = Some(n.clone());
        }
    }
    best
}
