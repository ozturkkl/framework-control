use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::error;
use crate::state::AppState;
use crate::config::{self, Config};

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


