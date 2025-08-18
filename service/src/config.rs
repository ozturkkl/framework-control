use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use tracing::info;
use crate::types::Config;

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


