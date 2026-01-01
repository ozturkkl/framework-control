use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use tracing::info;
use crate::types::Config;

pub fn config_path() -> PathBuf {
    // Explicit override always wins (all platforms)
    if let Ok(p) = std::env::var("FRAMEWORK_CONTROL_CONFIG") {
        return PathBuf::from(p);
    }

    // Windows: prefer ProgramData for system-wide service config
    #[cfg(windows)]
    {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".into());
        return PathBuf::from(base)
            .join("FrameworkControl")
            .join("config.json");
    }

    // Linux: system-wide config
    #[cfg(target_os = "linux")]
    {
        return PathBuf::from("/etc")
            .join("framework-control")
            .join("config.json");
    }

    // Unsupported platforms: make this explicit instead of silently picking a path.
    #[cfg(all(not(windows), not(target_os = "linux")))]
    {
        panic!("Unsupported platform: Framework Control currently supports Windows and Linux only");
    }
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
