use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;
use crate::state::AppState;
use crate::config::{self, Config};
use sysinfo::System;

pub async fn health() -> &'static str { "ok" }

#[derive(Serialize)]
pub struct CliOutput { pub ok: bool, pub stdout: Option<String>, pub error: Option<String> }

pub async fn get_power(State(state): State<AppState>) -> Json<CliOutput> {
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

pub async fn get_thermal(State(state): State<AppState>) -> Json<CliOutput> {
    let Some(cli) = &state.cli else {
        return Json(CliOutput { ok: false, stdout: None, error: Some("framework_tool not found".into()) });
    };
    match cli.thermal().await {
        Ok(output) => Json(CliOutput { ok: true, stdout: Some(output), error: None }),
        Err(e) => Json(CliOutput { ok: false, stdout: None, error: Some(e) }),
    }
}

pub async fn get_versions(State(state): State<AppState>) -> Json<CliOutput> {
    let Some(cli) = &state.cli else {
        return Json(CliOutput { ok: false, stdout: None, error: Some("framework_tool not found".into()) });
    };
    match cli.versions().await {
        Ok(output) => Json(CliOutput { ok: true, stdout: Some(output), error: None }),
        Err(e) => Json(CliOutput { ok: false, stdout: None, error: Some(e) }),
    }
}

// Removed direct fan duty endpoint; use POST /api/config with mode/manual_duty_pct instead

// Config endpoints (minimal)
#[derive(Serialize)]
pub struct ConfigEnvelope { pub ok: bool, pub config: Config }

pub async fn get_config(State(state): State<AppState>) -> Json<ConfigEnvelope> {
    let cfg = state.config.read().await.clone();
    Json(ConfigEnvelope { ok: true, config: cfg })
}

#[derive(Deserialize)]
pub struct UpdateConfig { pub config: Value }

#[derive(Serialize)]
pub struct UpdateResult { pub ok: bool }

pub async fn set_config(State(state): State<AppState>, Json(req): Json<UpdateConfig>) -> Json<UpdateResult> {
    // Merge incoming partial JSON with current config, then validate and persist
    let current_cfg = state.config.read().await.clone();
    let mut current_val = match serde_json::to_value(&current_cfg) { Ok(v) => v, Err(_) => Value::Null };
    deep_merge(&mut current_val, &req.config);
    let merged: Config = match serde_json::from_value(current_val) {
        Ok(c) => c,
        Err(e) => {
            error!("config merge/validation error: {}", e);
            return Json(UpdateResult { ok: false });
        }
    };
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

#[derive(Serialize)]
pub struct SystemInfoEnvelope {
    pub ok: bool,
    pub cpu: String,
    pub memory_total_mb: u64,
    pub os: String,
    pub dgpu: Option<String>,
}

pub async fn get_system_info() -> Json<SystemInfoEnvelope> {
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


fn deep_merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                deep_merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        // Arrays and scalars get replaced entirely
        (b, v) => { *b = v.clone(); }
    }
}

// Removed explicit fan-curve status endpoint; web can poll power/thermal


