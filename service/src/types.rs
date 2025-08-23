use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

// Core config types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Config {
    #[serde(default)]
    pub fan_curve: FanCurveConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fan_curve: FanCurveConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FanCurveConfig {
    #[serde(default)]
    pub mode: FanMode,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "FanCurveConfig::default_sensor")]
    pub sensor: String, // "APU" | "CPU"
    #[serde(default = "FanCurveConfig::default_points")]
    pub points: Vec<[u32; 2]>, // (temp_c, duty_pct)
    #[serde(default = "FanCurveConfig::default_poll_ms")]
    pub poll_ms: u64,
    #[serde(default = "FanCurveConfig::default_hysteresis_c")]
    pub hysteresis_c: u32,
    #[serde(default = "FanCurveConfig::default_rate_limit_pct_per_step")]
    pub rate_limit_pct_per_step: u32,
    #[serde(default)]
    pub manual_duty_pct: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<FanCalibration>,
}

impl FanCurveConfig {
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
}

impl Default for FanCurveConfig {
    fn default() -> Self {
        Self {
            mode: FanMode::default(),
            enabled: false,
            sensor: Self::default_sensor(),
            points: Self::default_points(),
            poll_ms: Self::default_poll_ms(),
            hysteresis_c: Self::default_hysteresis_c(),
            rate_limit_pct_per_step: Self::default_rate_limit_pct_per_step(),
            manual_duty_pct: None,
            calibration: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum, Default)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    #[default]
    Auto,
    Manual,
    Curve,
}

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
pub struct PartialConfig {
    pub fan_curve: Option<FanCurveConfig>,
}

// Fan calibration types
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FanCalibration {
    /// Calibration data points: [duty_pct, rpm]
    pub points: Vec<[u32; 2]>,
    /// Unix timestamp (seconds)
    pub updated_at: i64,
}
