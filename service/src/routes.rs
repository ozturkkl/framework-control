use std::collections::HashMap;

use crate::config; // for save/load
use crate::shortcuts;
use crate::types::TemperaturesResult;
use crate::update::{check_and_apply_now, get_current_and_latest};
use crate::state::AppState;
use crate::types::{
    CliOutput, ConfigEnvelope, PartialConfig, SystemInfoEnvelope, UpdateCheckEnvelope, UpdateResult,
};
use poem::web::Data;
use poem_openapi::{param::Header, payload::Json, OpenApi};
use serde_json::Value;
use sysinfo::System;
use tracing::{error, info};

pub struct Api;

#[OpenApi]
impl Api {
    /// Health: returns overall service health and CLI presence
    #[oai(path = "/health", method = "get", operation_id = "health")]
    async fn health(&self, state: Data<&AppState>) -> Json<Value> {
        let cli_present = state.cli.is_some();
        let service_version = env!("CARGO_PKG_VERSION");
        Json(serde_json::json!({
            "ok": true,
            "cli_present": cli_present,
            "serviceVersion": service_version,
        }))
    }

    /// Power info
    #[oai(path = "/power", method = "get", operation_id = "getPower")]
    async fn get_power(&self, state: Data<&AppState>) -> Json<CliOutput> {
        let Some(cli) = &state.cli else {
            return Json(CliOutput {
                ok: false,
                stdout: None,
                error: Some("framework_tool not found".into()),
            });
        };
        match cli.power().await {
            Ok(output) => Json(CliOutput {
                ok: true,
                stdout: Some(output),
                error: None,
            }),
            Err(e) => {
                error!("power exec error: {}", e);
                Json(CliOutput {
                    ok: false,
                    stdout: None,
                    error: Some(e),
                })
            }
        }
    }

    /// Update: check for latest version from update feed
    #[oai(path = "/update/check", method = "get", operation_id = "checkUpdate")]
    async fn check_update(&self) -> Json<UpdateCheckEnvelope> {
        match get_current_and_latest().await {
            Ok((current, latest)) => Json(UpdateCheckEnvelope { ok: true, current_version: current, latest_version: latest }),
            Err(e) => {
                error!("update check failed: {}", e);
                let current = env!("CARGO_PKG_VERSION").to_string();
                Json(UpdateCheckEnvelope { ok: false, current_version: current, latest_version: None })
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
    ) -> Json<UpdateResult> {
        let provided = auth.0.as_str().strip_prefix("Bearer ").unwrap_or("").trim();
        if !state.is_valid_token(Some(provided)) {
            return Json(UpdateResult { ok: false });
        }
        match check_and_apply_now().await {
            Ok(applied) => Json(UpdateResult { ok: applied }),
            Err(e) => {
                error!("apply update failed: {}", e);
                Json(UpdateResult { ok: false })
            }
        }
    }

    /// Thermal info
    #[oai(path = "/thermal", method = "get", operation_id = "getThermal")]
    async fn get_thermal(&self, state: Data<&AppState>) -> Json<TemperaturesResult> {
        let Some(cli) = &state.cli else {
            return Json(TemperaturesResult {
                ok: false,
                temps: HashMap::new()
            });
        };
        match cli.thermal().await {
            Ok(output) => Json(TemperaturesResult {
                ok: true,
                temps: parse_temperatures(output)
            }),
            Err(e) => {
                error!("apply update failed: {}", e);
                Json(TemperaturesResult {
                    ok: false,
                    temps: HashMap::new()
                })
            },
        }
    }

    /// Versions (from framework_tool CLI)
    #[oai(path = "/versions", method = "get", operation_id = "getVersions")]
    async fn get_versions(&self, state: Data<&AppState>) -> Json<CliOutput> {
        let Some(cli) = &state.cli else {
            return Json(CliOutput {
                ok: false,
                stdout: None,
                error: Some("framework_tool not found".into()),
            });
        };
        match cli.versions().await {
            Ok(output) => Json(CliOutput {
                ok: true,
                stdout: Some(output),
                error: None,
            }),
            Err(e) => Json(CliOutput {
                ok: false,
                stdout: None,
                error: Some(e),
            }),
        }
    }

    /// Get config
    #[oai(path = "/config", method = "get", operation_id = "getConfig")]
    async fn get_config(&self, state: Data<&AppState>) -> Json<ConfigEnvelope> {
        let cfg = state.config.read().await.clone();
        Json(ConfigEnvelope {
            ok: true,
            config: cfg,
        })
    }

    /// Set config (partial)
    #[oai(path = "/config", method = "post", operation_id = "setConfig")]
    async fn set_config(
        &self,
        state: Data<&AppState>,
        #[oai(name = "Authorization")] auth: Header<String>,
        req: Json<PartialConfig>,
    ) -> Json<UpdateResult> {
        let provided = auth.0.as_str().strip_prefix("Bearer ").unwrap_or("").trim();
        if !state.is_valid_token(Some(provided)) {
            return Json(UpdateResult { ok: false });
        }
        let req = req.0;
        let mut merged = state.config.read().await.clone();
        if let Some(fan) = req.fan {
            let mut new_fan = merged.fan.clone();
            // Overwrite sections only if provided
            if let Some(m) = fan.mode {
                new_fan.mode = m;
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
        if let Some(up) = req.updates {
            let mut new_up = merged.updates.clone();
            if let Some(ai) = up.auto_install { new_up.auto_install = ai; }
            merged.updates = new_up;
        }
        if let Err(e) = config::save(&merged) {
            error!("config save error: {}", e);
            return Json(UpdateResult { ok: false });
        }
        {
            let mut w = state.config.write().await;
            *w = merged;
        }
        info!("set_config applied successfully");
        Json(UpdateResult { ok: true })
    }

    /// System info
    #[oai(path = "/system", method = "get", operation_id = "getSystemInfo")]
    async fn get_system_info(&self) -> Json<SystemInfoEnvelope> {
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
        Json(SystemInfoEnvelope {
            ok: true,
            cpu,
            memory_total_mb: mem_mb,
            os,
            dgpu,
        })
    }

    #[oai(
        path = "/shortcuts/status",
        method = "get",
        operation_id = "getShortcutsStatus"
    )]
    async fn get_shortcuts_status(&self) -> Json<Value> {
        let installed = shortcuts::shortcuts_exist();
        Json(serde_json::json!({
            "ok": true,
            "installed": installed,
        }))
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
    ) -> Json<UpdateResult> {
        // Check auth
        let provided = auth.0.as_str().strip_prefix("Bearer ").unwrap_or("").trim();
        if !state.is_valid_token(Some(provided)) {
            return Json(UpdateResult { ok: false });
        }

        // Get port from environment (required at startup)
        let port: u16 = std::env::var("FRAMEWORK_CONTROL_PORT")
            .expect("FRAMEWORK_CONTROL_PORT must be set")
            .parse()
            .expect("FRAMEWORK_CONTROL_PORT must be valid");

        match shortcuts::create_shortcuts(port).await {
            Ok(_) => {
                info!("Shortcuts created successfully");
                Json(UpdateResult { ok: true })
            }
            Err(e) => {
                error!("Failed to create shortcuts: {}", e);
                Json(UpdateResult { ok: false })
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

/// Parse the temperature from the string given by frameworks' tools into a map
fn parse_temperatures(temperatures: String) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in temperatures.split(['\n', '\r']) {
        let trimmed_line = line.trim();
        if trimmed_line.len() > 0 {
            let parts = trimmed_line.split(":").collect::<Vec<&str>>();
            if parts.len() == 2 && parts[1].ends_with(['C', 'F']) {
                map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
            }
        }
    }
    return map;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_temperature() {
        let output = "  F75303_Local:   45 C\n  F75303_CPU:     55 C\n  APU:          62 C\n";
        let temps = parse_temperatures(output.to_string());

        assert_eq!(temps.get(&"F75303_Local".to_string()), Some(&"45 C".to_string()));
        assert_eq!(temps.get(&"F75303_CPU".to_string()), Some(&"55 C".to_string()));
        assert_eq!(temps.get(&"APU".to_string()), Some(&"62 C".to_string()));
        assert_eq!(temps.get(&"dwada".to_string()), None);
    }
}