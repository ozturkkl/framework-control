use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

// Core config types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Config {
    #[serde(default)]
    pub fan: FanControlConfig,
    #[serde(default)]
    pub power: PowerConfig,
    #[serde(default)]
    pub updates: UpdatesConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fan: FanControlConfig::default(),
            power: PowerConfig::default(),
            updates: UpdatesConfig::default(),
        }
    }
}

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
    pub mode: Option<FanControlMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual: Option<ManualConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub curve: Option<CurveConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<FanCalibration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ManualConfig {
    pub duty_pct: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct CurveConfig {
    #[serde(default = "default_sensor")]
    pub sensor: String,
    #[serde(default = "default_points")]
    pub points: Vec<[u32; 2]>,
    #[serde(default = "default_poll_ms")]
    pub poll_ms: u64,
    #[serde(default = "default_hysteresis_c")]
    pub hysteresis_c: u32,
    #[serde(default = "default_rate_limit_pct_per_step")]
    pub rate_limit_pct_per_step: u32,
}

fn default_sensor() -> String {
    "APU".to_string()
}
fn default_points() -> Vec<[u32; 2]> {
    vec![[40, 0], [60, 40], [75, 80], [85, 100]]
}
fn default_poll_ms() -> u64 {
    2000
}
fn default_hysteresis_c() -> u32 {
    2
}
fn default_rate_limit_pct_per_step() -> u32 {
    100
}

#[derive(Serialize, Object)]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: String,
}

#[derive(Serialize, Object)]
pub struct SystemInfo {
    pub cpu: String,
    pub memory_total_mb: u64,
    pub os: String,
    pub dgpu: Option<String>,
}

#[derive(Serialize, Object)]
pub struct Health {
    pub cli_present: bool,
    pub service_version: String,
}

#[derive(Serialize, Object, Default)]
pub struct ShortcutsStatus {
    pub installed: bool,
}

#[derive(Serialize, Object, Default)]
pub struct Empty {}

#[derive(Debug, Clone, Deserialize, Object)]
pub struct PartialConfig {
    pub fan: Option<FanControlConfig>,
    pub power: Option<PowerConfig>,
    pub updates: Option<UpdatesConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct UpdatesConfig {
    #[serde(default)]
    pub auto_install: bool,
}

// Fan calibration types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FanCalibration {
    /// Calibration data points: [duty_pct, rpm]
    pub points: Vec<[u32; 2]>,
    /// Unix timestamp (seconds)
    pub updated_at: i64,
}

// Generic API error envelope
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ErrorEnvelope {
    pub code: String,
    pub message: String,
}

// Power config stored in Config and applied at boot (and on set)
#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct PowerConfig {
    /// Desired package power in Watts (applied to STAPM/FAST/SLOW equally)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tdp_watts: Option<u32>,
    /// Tctl thermal limit in degrees Celsius
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_limit_c: Option<u32>,
}

// Combined power response used by /power
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct PowerResponse {
    /// Framework CLI power fields (flattened)
    #[oai(flatten)]
    pub power: crate::cli::framework_tool_parser::PowerParsed,
    /// RyzenAdj presence and parsed info
    pub ryzenadj_installed: bool,
    #[oai(flatten)]
    #[oai(skip_serializing_if = "Option::is_none")]
    pub ryzenadj: Option<crate::cli::ryzen_adj_parser::RyzenAdjInfo>,
}