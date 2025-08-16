use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub fan_curve: FanCurveConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self { fan_curve: FanCurveConfig::default() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurveConfig {
    #[serde(default = "default_mode")] 
    pub mode: FanMode,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_sensor")] 
    pub sensor: String, // "APU" | "CPU"
    #[serde(default = "default_points")] 
    pub points: Vec<(u32, u32)>, // (temp_c, duty_pct)
    #[serde(default = "default_poll_ms")] 
    pub poll_ms: u64,
    #[serde(default = "default_hysteresis_c")] 
    pub hysteresis_c: u32,
    #[serde(default = "default_rate_limit")] 
    pub rate_limit_pct_per_step: u32,
    #[serde(default)]
    pub manual_duty_pct: Option<u32>,
}

impl Default for FanCurveConfig {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            enabled: false,
            sensor: default_sensor(),
            points: default_points(),
            poll_ms: default_poll_ms(),
            hysteresis_c: default_hysteresis_c(),
            rate_limit_pct_per_step: default_rate_limit(),
            manual_duty_pct: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanMode { Auto, Manual, Curve }

impl Default for FanMode { fn default() -> Self { FanMode::Auto } }

fn default_mode() -> FanMode { FanMode::Auto }
fn default_sensor() -> String { "APU".to_string() }
fn default_points() -> Vec<(u32, u32)> { vec![(40, 0), (60, 40), (75, 80), (85, 100)] }
fn default_poll_ms() -> u64 { 2000 }
fn default_hysteresis_c() -> u32 { 2 }
fn default_rate_limit() -> u32 { 100 }

pub fn config_path() -> PathBuf {
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }
    // Prefer ProgramData for system-wide service config
    let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
    PathBuf::from(base).join("FrameworkControl").join("config.json")
}

pub fn load() -> Config {
    let path = config_path();
    if let Ok(mut f) = File::open(&path) {
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_ok() {
            if let Ok(cfg) = serde_json::from_str::<Config>(&buf) {
                info!("Loaded config from {:?}", path);
                return cfg;
            }
        }
    }
    Config::default()
}

pub fn save(cfg: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(dir) = path.parent() { create_dir_all(dir).map_err(|e| e.to_string())?; }
    let mut f = File::create(&path).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())
}


