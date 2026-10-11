use super::*;
use serde_json::json;

fn config(value: serde_json::Value) -> Config {
    serde_json::from_value(value).unwrap()
}

fn patch(value: serde_json::Value) -> PartialConfig {
    serde_json::from_value(value).unwrap()
}

#[test]
fn fan_overrides_kept_when_omitted_and_cleared_by_empty_list() {
    let mut cfg =
        config(json!({ "fan": { "mode": "curve", "overrides": [{ "index": 1, "manual": { "duty_pct": 50 } }] } }));

    apply_partial(&mut cfg, patch(json!({ "fan": { "mode": "manual" } })));
    assert_eq!(cfg.fan.overrides.as_ref().map(Vec::len), Some(1));

    apply_partial(&mut cfg, patch(json!({ "fan": { "overrides": [] } })));
    assert!(cfg.fan.overrides.is_none());
}

#[test]
fn power_patch_merges_single_field_into_existing_profiles() {
    let mut cfg = config(json!({ "power": {
        "ac": { "tdp_watts": { "enabled": true, "value": 45 } },
        "battery": { "tdp_watts": { "enabled": true, "value": 15 } },
    } }));

    apply_partial(
        &mut cfg,
        patch(json!({ "power": { "ac": { "thermal_limit_c": { "enabled": true, "value": 90 } } } })),
    );

    let ac = cfg.power.ac.unwrap();
    assert_eq!(ac.tdp_watts.unwrap().value, 45);
    assert_eq!(ac.thermal_limit_c.unwrap().value, 90);
    assert_eq!(cfg.power.battery.unwrap().tdp_watts.unwrap().value, 15);
}

#[test]
fn charge_rate_patch_without_threshold_clears_soc_threshold() {
    let mut cfg = config(json!({ "battery": {
        "charge_limit_max_pct": { "enabled": true, "value": 80 },
        "charge_rate_c": { "enabled": true, "value": 0.5 },
        "charge_rate_soc_threshold_pct": 60,
    } }));

    apply_partial(
        &mut cfg,
        patch(json!({ "battery": { "charge_rate_c": { "enabled": true, "value": 0.3 } } })),
    );

    assert_eq!(cfg.battery.charge_rate_c.unwrap().value, 0.3);
    assert_eq!(cfg.battery.charge_rate_soc_threshold_pct, None);
    assert_eq!(cfg.battery.charge_limit_max_pct.unwrap().value, 80);
}
