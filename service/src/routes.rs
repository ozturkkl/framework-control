use crate::config; // for save/load
use crate::shortcuts;
use crate::state::AppState;
use crate::types::{Empty, Health, PartialConfig, ShortcutsStatus, SystemInfo, UpdateCheck};
use crate::update::{check_and_apply_now, get_current_and_latest};
use poem::web::Data;
use poem_openapi::{param::Header, payload::Json, ApiResponse, OpenApi};
use serde_json::Value;
use sysinfo::System;
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
async fn require_ryzenadj_async(
    state: &AppState,
) -> Result<crate::cli::ryzen_adj::RyzenAdj, ApiErrorResponse> {
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
async fn require_linux_power_async(
    state: &AppState,
) -> Result<crate::cli::linux_power::LinuxPower, ApiErrorResponse> {
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

fn bearer_from_header(auth: &Header<String>) -> &str {
    auth.0.as_str().strip_prefix("Bearer ").unwrap_or("").trim()
}

fn require_auth(state: &AppState, auth: &Header<String>) -> Result<(), ApiErrorResponse> {
    let provided = bearer_from_header(auth);
    if state.is_valid_token(Some(provided)) {
        Ok(())
    } else {
        Err(bad_gateway("unauthorized", "invalid token".into()))
    }
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
    #[oai(
        path = "/ryzenadj/install",
        method = "post",
        operation_id = "installRyzenadj"
    )]
    async fn install_ryzenadj(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
    ) -> ApiResult<Empty> {
        require_auth(&state, &auth)?;
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
            Err(bad_gateway("unsupported_platform", "RyzenAdj is only available on Windows. Linux uses native kernel interfaces.".to_string()))
        }
    }

    /// RyzenAdj: uninstall and remove any downloaded artifacts (Windows only)
    #[oai(
        path = "/ryzenadj/uninstall",
        method = "post",
        operation_id = "uninstallRyzenadj"
    )]
    async fn uninstall_ryzenadj(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
    ) -> ApiResult<Empty> {
        require_auth(&state, &auth)?;
        #[cfg(target_os = "windows")]
        {
            match crate::cli::ryzen_adj::remove_installed_files().await {
                Ok(_) => {
                    // Clear from in-memory state so UI reflects removal soon
                    {
                        let mut w = state.ryzenadj.write().await;
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
            Err(bad_gateway("unsupported_platform", "RyzenAdj is only available on Windows. Linux uses native kernel interfaces.".to_string()))
        }
    }

    #[oai(path = "/power", method = "get", operation_id = "getPower")]
    async fn get_power(&self, state: Data<&AppState>) -> ApiResult<crate::types::PowerResponse> {
        let cli = require_framework_tool_async(&state).await?;
        let p = cli.power().await.map_err(map_cli_err)?;

        // Also include charge limit min/max when available; do not fail if missing
        let limits = match cli.charge_limit_get().await {
            Ok(info) => info,
            Err(_e) => Default::default(),
        };
        // Build API-facing battery info by combining parsed battery + limits (always include)
        let battery_api: Option<crate::types::BatteryInfo> = Some(crate::types::BatteryInfo {
            power_info: p.clone(),
            limits,
        });

        // Get power control info based on platform
        let power_control = {
            #[cfg(target_os = "windows")]
            {
                if let Ok(ryz) = require_ryzenadj_async(&state).await {
                    let capabilities = ryz.get_capabilities();
                    let current_state = ryz.get_state().await.unwrap_or_default();
                    crate::types::PowerControlInfo {
                        method: "ryzenadj".to_string(),
                        capabilities,
                        current_state,
                    }
                } else {
                    crate::types::PowerControlInfo {
                        method: "none".to_string(),
                        capabilities: Default::default(),
                        current_state: Default::default(),
                    }
                }
            }

            #[cfg(target_os = "linux")]
            {
                if let Ok(lp) = require_linux_power_async(&state).await {
                    let method = lp.method_name();
                    let capabilities = lp.get_capabilities().await;
                    let current_state = lp.get_state().await.unwrap_or_default();
                    crate::types::PowerControlInfo {
                        method,
                        capabilities,
                        current_state,
                    }
                } else {
                    crate::types::PowerControlInfo {
                        method: "none".to_string(),
                        capabilities: Default::default(),
                        current_state: Default::default(),
                    }
                }
            }

            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            {
                crate::types::PowerControlInfo {
                    method: "none".to_string(),
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

    /// Update: check for latest version from update feed
    #[oai(path = "/update/check", method = "get", operation_id = "checkUpdate")]
    async fn check_update(&self) -> ApiResult<UpdateCheck> {
        match get_current_and_latest().await {
            Ok((current, latest)) => Ok(Json(UpdateCheck {
                current_version: current,
                latest_version: latest,
            })),
            Err(e) => {
                error!("update check failed: {}", e);
                Err(bad_gateway("update_check_failed", e))
            }
        }
    }

    /// Update: apply latest by downloading MSI and invoking msiexec (Windows only)
    #[oai(path = "/update/apply", method = "post", operation_id = "applyUpdate")]
    async fn apply_update(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
        _req: Json<Value>,
    ) -> ApiResult<Empty> {
        require_auth(&state, &auth)?;
        match check_and_apply_now().await {
            Ok(_applied) => Ok(Json(Empty {})),
            Err(e) => {
                error!("apply update failed: {}", e);
                Err(bad_gateway("apply_failed", e))
            }
        }
    }

    /// Thermal (parsed)
    #[oai(path = "/thermal", method = "get", operation_id = "getThermal")]
    async fn get_thermal(
        &self,
        state: Data<&AppState>,
    ) -> ApiResult<crate::cli::framework_tool_parser::ThermalParsed> {
        let cli = require_framework_tool_async(&state).await?;
        let v = cli.thermal().await.map_err(map_cli_err)?;
        Ok(Json(v))
    }

    /// Telemetry history: returns recent samples collected by the service
    #[oai(
        path = "/thermal/history",
        method = "get",
        operation_id = "getThermalHistory"
    )]
    async fn get_thermal_history(
        &self,
        state: Data<&AppState>,
    ) -> ApiResult<Vec<crate::types::TelemetrySample>> {
        let samples: Vec<crate::types::TelemetrySample> = {
            let r = state.telemetry_samples.read().await;
            r.iter().cloned().collect()
        };
        Ok(Json(samples))
    }

    /// Framework versions (parsed)
    #[oai(path = "/versions", method = "get", operation_id = "getVersions")]
    async fn get_versions(
        &self,
        state: Data<&AppState>,
    ) -> ApiResult<crate::cli::framework_tool_parser::VersionsParsed> {
        let cli = require_framework_tool_async(&state).await?;
        let v = cli.versions().await.map_err(map_cli_err)?;
        Ok(Json(v))
    }

    /// Get config
    #[oai(path = "/config", method = "get", operation_id = "getConfig")]
    async fn get_config(&self, state: Data<&AppState>) -> ApiResult<crate::types::Config> {
        let cfg = state.config.read().await.clone();
        Ok(Json(cfg))
    }

    /// Set config (partial)
    #[oai(path = "/config", method = "post", operation_id = "setConfig")]
    async fn set_config(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
        req: Json<PartialConfig>,
    ) -> ApiResult<Empty> {
        require_auth(&state, &auth)?;
        let req = req.0;
        let mut merged = state.config.read().await.clone();
        if let Some(fan) = req.fan {
            let mut new_fan = merged.fan.clone();
            // Overwrite sections only if provided
            if let Some(m) = fan.mode {
                new_fan.mode = Some(m);
            }
            if let Some(man) = fan.manual {
                new_fan.manual = Some(man);
            }
            if let Some(cur) = fan.curve {
                new_fan.curve = Some(cur);
            }
            if let Some(cal) = fan.calibration {
                new_fan.calibration = Some(cal);
            }
            merged.fan = new_fan;
        }
        if let Some(pow) = req.power {
            let mut new_pow = merged.power.clone();
            if let Some(ac_in) = pow.ac {
                let mut ac = new_pow.ac.unwrap_or_default();
                if let Some(s) = ac_in.tdp_watts {
                    ac.tdp_watts = Some(s);
                }
                if let Some(s) = ac_in.thermal_limit_c {
                    ac.thermal_limit_c = Some(s);
                }
                if let Some(s) = ac_in.epp_preference {
                    ac.epp_preference = Some(s);
                }
                if let Some(s) = ac_in.governor {
                    ac.governor = Some(s);
                }
                if let Some(s) = ac_in.min_freq_mhz {
                    ac.min_freq_mhz = Some(s);
                }
                if let Some(s) = ac_in.max_freq_mhz {
                    ac.max_freq_mhz = Some(s);
                }
                new_pow.ac = Some(ac);
            }
            if let Some(bat_in) = pow.battery {
                let mut bat = new_pow.battery.unwrap_or_default();
                if let Some(s) = bat_in.tdp_watts {
                    bat.tdp_watts = Some(s);
                }
                if let Some(s) = bat_in.thermal_limit_c {
                    bat.thermal_limit_c = Some(s);
                }
                if let Some(s) = bat_in.epp_preference {
                    bat.epp_preference = Some(s);
                }
                if let Some(s) = bat_in.governor {
                    bat.governor = Some(s);
                }
                if let Some(s) = bat_in.min_freq_mhz {
                    bat.min_freq_mhz = Some(s);
                }
                if let Some(s) = bat_in.max_freq_mhz {
                    bat.max_freq_mhz = Some(s);
                }
                new_pow.battery = Some(bat);
            }
            merged.power = new_pow;
        }
        if let Some(up) = req.updates {
            let mut new_up = merged.updates.clone();
            new_up.auto_install = up.auto_install;
            merged.updates = new_up;
        }
        if let Some(bat) = req.battery {
            let mut new_bat = merged.battery.clone();
            if let Some(s) = bat.charge_limit_max_pct {
                new_bat.charge_limit_max_pct = Some(s);
            }
            if let Some(s) = bat.charge_rate_c {
                new_bat.charge_rate_c = Some(s);
                new_bat.charge_rate_soc_threshold_pct = bat.charge_rate_soc_threshold_pct;
            }
            merged.battery = new_bat;
        }
        if let Some(tel) = req.telemetry {
            merged.telemetry = tel;
        }
        if let Some(ui) = req.ui {
            let mut new_ui = merged.ui.clone();
            if let Some(theme) = ui.theme {
                new_ui.theme = Some(theme);
            }
            merged.ui = new_ui;
        }
        if let Err(e) = config::save(&merged) {
            error!("config save error: {}", e);
            return Err(bad_gateway("save_failed", e));
        }
        {
            let mut w = state.config.write().await;
            *w = merged;
        }
        info!("set_config applied successfully");
        Ok(Json(Empty {}))
    }

    /// System info
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

    #[oai(
        path = "/shortcuts/status",
        method = "get",
        operation_id = "getShortcutsStatus"
    )]
    async fn get_shortcuts_status(&self) -> ApiResult<ShortcutsStatus> {
        let installed = shortcuts::shortcuts_exist();
        Ok(Json(ShortcutsStatus { installed }))
    }

    #[oai(
        path = "/shortcuts/create",
        method = "post",
        operation_id = "createShortcuts"
    )]
    async fn create_shortcuts(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
    ) -> ApiResult<Empty> {
        // Check auth
        require_auth(&state, &auth)?;

        // Get port from environment (required at startup)
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

    /// Logs: retrieve recent service logs
    #[oai(path = "/logs", method = "get", operation_id = "getLogs")]
    async fn get_logs(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
    ) -> Result<poem_openapi::payload::PlainText<String>, ApiErrorResponse> {
        require_auth(&state, &auth)?;

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
        // Read FrameworkControlService.out.log from the service directory
        let exe = std::env::current_exe()
            .map_err(|e| format!("failed to get current exe path: {}", e))?;
        let dir = exe.parent()
            .ok_or_else(|| "failed to get exe directory".to_string())?;
        let log_path = dir.join("FrameworkControlService.out.log");

        let contents = std::fs::read_to_string(&log_path)
            .map_err(|e| format!("failed to read log file: {}", e))?;

        // Return last 500 lines (approximate)
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
        let looks_integrated =
            lo.contains("uhd") || lo.contains("iris") || lo.contains("vega") || lo.contains("780m");
        if looks_discrete && !looks_integrated {
            return Some(n.clone());
        }
        if best.is_none() && !looks_integrated {
            best = Some(n.clone());
        }
    }
    best
}
