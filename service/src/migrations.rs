use serde_json::{json, Map, Value};

pub const CURRENT_VERSION: u32 = 1;

/// Upgrades a raw config file in place; returns true if it changed.
pub fn migrate(cfg: &mut Map<String, Value>) -> bool {
    let version = cfg.get("version").and_then(Value::as_u64).unwrap_or(0);
    if version >= CURRENT_VERSION as u64 {
        return false;
    }
    if version < 1 {
        fan_settings_into_profile(cfg);
    }
    cfg.insert("version".into(), json!(CURRENT_VERSION));
    true
}

/// v1: top-level fan mode/manual/curve/overrides move into a single "Default" profile.
fn fan_settings_into_profile(cfg: &mut Map<String, Value>) {
    let Some(fan) = cfg.get_mut("fan").and_then(Value::as_object_mut) else {
        return;
    };
    if fan.contains_key("profiles") {
        return;
    }
    let mut profile = json!({ "id": "default", "name": "Default" });
    for key in ["mode", "manual", "curve", "overrides"] {
        if let Some(value) = fan.remove(key).filter(|v| !v.is_null()) {
            profile[key] = value;
        }
    }
    fan.insert("profiles".into(), json!([profile]));
    fan.insert("active_profile".into(), json!("default"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Config, FanControlMode};

    fn migrated(value: Value) -> Config {
        let Value::Object(mut raw) = value else { unreachable!() };
        assert!(migrate(&mut raw));
        serde_json::from_value(Value::Object(raw)).unwrap()
    }

    #[test]
    fn legacy_fan_settings_become_default_profile() {
        let cfg = migrated(json!({
            "fan": {
                "mode": "manual",
                "manual": { "duty_pct": 42 },
                "calibration": { "updated_at": 1, "fans": [] }
            }
        }));
        assert_eq!(cfg.version, CURRENT_VERSION);
        assert_eq!(cfg.fan.active_profile, "default");
        let profile = cfg.fan.active().unwrap();
        assert_eq!(profile.mode, FanControlMode::Manual);
        assert_eq!(profile.manual.as_ref().unwrap().duty_pct, 42);
        assert!(cfg.fan.calibration.is_some());
    }

    #[test]
    fn missing_fan_section_uses_defaults() {
        let cfg = migrated(json!({}));
        assert!(cfg.fan.validate().is_ok());
    }

    #[test]
    fn existing_profiles_survive_rerun() {
        let cfg = migrated(json!({
            "fan": {
                "profiles": [{ "id": "quiet", "name": "Quiet", "mode": "manual" }],
                "active_profile": "quiet"
            }
        }));
        assert_eq!(cfg.fan.profiles.len(), 1);
        assert_eq!(cfg.fan.active().unwrap().id, "quiet");
    }

    #[test]
    fn current_version_is_untouched() {
        let Value::Object(mut raw) = json!({ "version": CURRENT_VERSION, "fan": { "mode": "manual" } }) else {
            unreachable!()
        };
        assert!(!migrate(&mut raw));
        assert_eq!(raw["fan"]["mode"], "manual");
    }
}
