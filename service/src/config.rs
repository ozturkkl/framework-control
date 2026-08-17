use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock, RwLockReadGuard};
use tracing::info;

use crate::types::{Config, ConfigEvent, PartialConfig};

#[derive(Clone)]
pub struct LiveConfig {
    inner: Arc<RwLock<Config>>,
    events: broadcast::Sender<ConfigEvent>,
}

impl LiveConfig {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(16);
        Self {
            inner: Arc::new(RwLock::new(load())),
            events,
        }
    }

    pub async fn read(&self) -> RwLockReadGuard<'_, Config> {
        self.inner.read().await
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ConfigEvent> {
        self.events.subscribe()
    }

    /// Merge, persist, replace, and broadcast under one write lock.
    pub async fn persist(&self, client_id: Option<String>, apply: impl FnOnce(&mut Config)) -> Result<Config, String> {
        let mut w = self.inner.write().await;
        let mut cfg = w.clone();
        apply(&mut cfg);
        save(&cfg)?;
        *w = cfg.clone();
        let _ = self.events.send(ConfigEvent {
            client_id,
            config: cfg.clone(),
        });
        Ok(cfg)
    }

    pub async fn persist_partial(&self, client_id: Option<String>, req: PartialConfig) -> Result<Config, String> {
        self.persist(client_id, |cfg| apply_partial(cfg, req)).await
    }
}

fn apply_partial(merged: &mut Config, req: PartialConfig) {
    if let Some(fan) = req.fan {
        let mut new_fan = merged.fan.clone();
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
        // Overrides are replaced wholesale when provided. An empty array clears them entirely.
        if let Some(ov) = fan.overrides {
            new_fan.overrides = if ov.is_empty() { None } else { Some(ov) };
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
}

fn config_path() -> PathBuf {
    // Explicit override always wins (all platforms)
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }

    // Windows: prefer ProgramData for system-wide service config
    #[cfg(windows)]
    {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
        return PathBuf::from(base).join("FrameworkControl").join("config.json");
    }

    // Linux: system-wide config
    #[cfg(target_os = "linux")]
    {
        return PathBuf::from("/etc").join("framework-control").join("config.json");
    }

    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        panic!("Unsupported platform: Framework Control currently supports Windows and Linux only");
    }
}

fn load() -> Config {
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

fn save(cfg: &Config) -> Result<(), String> {
    let path = config_path();
    if let Some(dir) = path.parent() {
        create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut f = File::create(&path).map_err(|e| e.to_string())?;
    let s = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    f.write_all(s.as_bytes()).map_err(|e| e.to_string())
}
