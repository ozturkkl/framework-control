use poem::web::Data;
use poem_openapi::{payload::Json, OpenApi, payload::PlainText};
use tracing::error;
use crate::state::AppState;
use crate::config; // for save/load
use crate::types::{CliOutput, ConfigEnvelope, UpdateResult, SystemInfoEnvelope, PartialConfig};
use sysinfo::System;

pub struct Api;

#[OpenApi]
impl Api {
    /// Health
    #[oai(path = "/api/health", method = "get", operation_id = "health")]
    async fn health(&self) -> PlainText<&'static str> { PlainText("ok") }

    /// Power info
    #[oai(path = "/api/power", method = "get", operation_id = "getPower")]
    async fn get_power(&self, state: Data<&AppState>) -> Json<CliOutput> {
        let Some(cli) = &state.cli else {
            return Json(CliOutput { ok: false, stdout: None, error: Some("framework_tool not found".into()) });
        };
        match cli.power().await {
            Ok(output) => Json(CliOutput { ok: true, stdout: Some(output), error: None }),
            Err(e) => {
                error!("power exec error: {}", e);
                Json(CliOutput { ok: false, stdout: None, error: Some(e) })
            }
        }
    }

    /// Thermal info
    #[oai(path = "/api/thermal", method = "get", operation_id = "getThermal")]
    async fn get_thermal(&self, state: Data<&AppState>) -> Json<CliOutput> {
        let Some(cli) = &state.cli else {
            return Json(CliOutput { ok: false, stdout: None, error: Some("framework_tool not found".into()) });
        };
        match cli.thermal().await {
            Ok(output) => Json(CliOutput { ok: true, stdout: Some(output), error: None }),
            Err(e) => Json(CliOutput { ok: false, stdout: None, error: Some(e) }),
        }
    }

    /// Versions
    #[oai(path = "/api/versions", method = "get", operation_id = "getVersions")]
    async fn get_versions(&self, state: Data<&AppState>) -> Json<CliOutput> {
        let Some(cli) = &state.cli else {
            return Json(CliOutput { ok: false, stdout: None, error: Some("framework_tool not found".into()) });
        };
        match cli.versions().await {
            Ok(output) => Json(CliOutput { ok: true, stdout: Some(output), error: None }),
            Err(e) => Json(CliOutput { ok: false, stdout: None, error: Some(e) }),
        }
    }

    /// Get config
    #[oai(path = "/api/config", method = "get", operation_id = "getConfig")]
    async fn get_config(&self, state: Data<&AppState>) -> Json<ConfigEnvelope> {
        let cfg = state.config.read().await.clone();
        Json(ConfigEnvelope { ok: true, config: cfg })
    }

    /// Set config (partial)
    #[oai(path = "/api/config", method = "post", operation_id = "setConfig")]
    async fn set_config(&self, state: Data<&AppState>, req: Json<PartialConfig>) -> Json<UpdateResult> {
        let req = req.0;
        let mut merged = state.config.read().await.clone();
        if let Some(fc) = req.fan_curve { merged.fan_curve = fc; }
        if let Err(e) = config::save(&merged) {
            error!("config save error: {}", e);
            return Json(UpdateResult { ok: false });
        }
        {
            let mut w = state.config.write().await;
            *w = merged;
        }
        Json(UpdateResult { ok: true })
    }

    /// System info
    #[oai(path = "/api/system", method = "get", operation_id = "getSystemInfo")]
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
        Json(SystemInfoEnvelope { ok: true, cpu, memory_total_mb: mem_mb, os, dgpu })
    }
}

async fn get_gpu_names() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        use tokio::process::Command;
        let ps = "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";
        if let Ok(out) = Command::new("powershell")
            .arg("-NoProfile").arg("-NonInteractive").arg("-Command").arg(ps)
            .output().await
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
        let looks_discrete = lo.contains("rtx") || lo.contains("gtx") || lo.contains("rx ") || lo.contains("arc ") || lo.contains("radeon pro") || lo.contains("geforce") || lo.contains("quadro") || lo.contains("radeon rx");
        let looks_integrated = lo.contains("uhd") || lo.contains("iris") || lo.contains("vega") || lo.contains("780m");
        if looks_discrete && !looks_integrated { return Some(n.clone()); }
        if best.is_none() && !looks_integrated { best = Some(n.clone()); }
    }
    best
}



