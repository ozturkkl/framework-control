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

    /// RyzenAdj: install on demand
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

    /// RyzenAdj: uninstall and remove any downloaded artifacts
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

    #[oai(path = "/power", method = "get", operation_id = "getPower")]
    async fn get_power(&self, state: Data<&AppState>) -> ApiResult<crate::types::PowerResponse> {
        let cli = require_framework_tool_async(&state).await?;
        let p = cli.power().await.map_err(map_cli_err)?;

        // Populate RyzenAdj info if available; do not fail the call if missing
        let mut ryzenadj_installed = false;
        let mut ryzenadj: Option<crate::cli::ryzen_adj_parser::RyzenAdjInfo> = None;
        if let Ok(ryz) = require_ryzenadj_async(&state).await {
            if let Ok(parsed) = ryz.info().await {
                ryzenadj_installed = true;
                ryzenadj = Some(parsed);
            }
        }
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
        Ok(Json(crate::types::PowerResponse {
            battery: battery_api,
            ryzenadj_installed,
            ryzenadj,
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
            }
            // SoC threshold: UI always sends this field; null clears, number sets.
            new_bat.charge_rate_soc_threshold_pct = bat.charge_rate_soc_threshold_pct;
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
            .expect("FRAMEWORK_CONTROL_PORT must be set")
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
