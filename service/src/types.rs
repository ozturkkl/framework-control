use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

// Core config types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Config {
    #[serde(default)]
    pub fan: FanControlConfig,
}

impl Default for Config { fn default() -> Self { Self { fan: FanControlConfig::default() } } }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum, Default)]
#[serde(rename_all = "lowercase")]
pub enum FanControlMode {
    #[default]
    #[oai(rename = "disabled")]
    Disabled,
    #[oai(rename = "manual")]
    Manual,
    #[oai(rename = "curve")]
    Curve,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct FanControlConfig {
    #[serde(default)]
    pub mode: FanControlMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual: Option<ManualConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
    /// Optional calibration at the root to allow updates without touching curve config
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<FanCalibration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ManualConfig { pub duty_pct: u32 }

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct CurveConfig {
    #[serde(default = "default_sensor")] pub sensor: String,
    #[serde(default = "default_points")] pub points: Vec<[u32; 2]>,
    #[serde(default = "default_poll_ms")] pub poll_ms: u64,
    #[serde(default = "default_hysteresis_c")] pub hysteresis_c: u32,
    #[serde(default = "default_rate_limit_pct_per_step")] pub rate_limit_pct_per_step: u32,
    #[serde(skip_serializing_if = "Option::is_none")] pub calibration: Option<FanCalibration>,
}

fn default_sensor() -> String { "APU".to_string() }
fn default_points() -> Vec<[u32; 2]> { vec![[40, 0], [60, 40], [75, 80], [85, 100]] }
fn default_poll_ms() -> u64 { 2000 }
fn default_hysteresis_c() -> u32 { 2 }
fn default_rate_limit_pct_per_step() -> u32 { 100 }

// API envelope types
#[derive(Serialize, Object)]
pub struct CliOutput {
    pub ok: bool,
    pub stdout: Option<String>,
    pub error: Option<String>,
}

#[derive(Serialize, Object)]
pub struct ConfigEnvelope {
    pub ok: bool,
    pub config: Config,
}

#[derive(Serialize, Object)]
pub struct UpdateResult {
    pub ok: bool,
}

#[derive(Serialize, Object)]
pub struct SystemInfoEnvelope {
    pub ok: bool,
    pub cpu: String,
    pub memory_total_mb: u64,
    pub os: String,
    pub dgpu: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Object)]
pub struct PartialConfig { pub fan: Option<PartialFanControlConfig> }

#[derive(Debug, Clone, Deserialize, Object, Default)]
pub struct PartialFanControlConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<FanControlMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual: Option<ManualConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<FanCalibration>,
}

// Fan calibration types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FanCalibration {
    /// Calibration data points: [duty_pct, rpm]
    pub points: Vec<[u32; 2]>,
    /// Unix timestamp (seconds)
    pub updated_at: i64,
}
