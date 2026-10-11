use super::*;
use serde_json::json;

// A parse failure makes config::load fall back to defaults, silently wiping the user's settings.
#[test]
fn config_file_from_0_5_4_keeps_user_settings() {
    let legacy = json!({
        "fan": {
            "mode": "curve",
            "curve": {
                "sensors": ["APU"],
                "points": [[50, 20], [80, 100]],
                "poll_ms": 3000,
                "hysteresis_c": 3,
                "rate_limit_pct_per_step": 10
            },
            "calibration": { "points": [[20, 1500], [100, 5000]], "updated_at": 1700000000 },
            "overrides": [{ "index": 1, "manual": { "duty_pct": 40 } }]
        },
        "power": { "ac": { "tdp_watts": { "enabled": true, "value": 45 } } },
        "battery": { "charge_limit_max_pct": { "enabled": true, "value": 80 } },
        "updates": { "auto_install": true },
        "telemetry": { "poll_ms": 2000, "retain_seconds": 1800 },
        "ui": { "theme": "dark" },
        "framework_tool": { "recommended": false }
    });

    let cfg: Config = serde_json::from_value(legacy).unwrap();

    let curve = cfg.fan.curve.unwrap();
    assert_eq!(curve.poll_ms, 3000);
    assert_eq!(curve.curve.sensors, vec!["APU"]);
    assert_eq!(curve.curve.points, vec![[50, 20], [80, 100]]);
    assert!(cfg.fan.calibration.unwrap().fans.is_empty());
    assert_eq!(cfg.fan.overrides.unwrap()[0].manual.as_ref().unwrap().duty_pct, 40);
    assert_eq!(cfg.power.ac.unwrap().tdp_watts.unwrap().value, 45);
    assert_eq!(cfg.battery.charge_limit_max_pct.unwrap().value, 80);
    assert!(cfg.updates.auto_install);
    assert_eq!(cfg.ui.theme.as_deref(), Some("dark"));
    assert!(!cfg.framework_tool.latest);
}

#[test]
fn panels_missing_from_layout_stay_enabled() {
    let no_layout = UiConfig::default();
    assert!(no_layout.is_panel_enabled(DashboardPanelId::Fan));

    let layout = UiConfig {
        panels: Some(vec![DashboardPanel {
            id: DashboardPanelId::Fan,
            enabled: false,
            size: DashboardPanelSize::Half,
        }]),
        ..Default::default()
    };
    assert!(!layout.is_panel_enabled(DashboardPanelId::Fan));
    assert!(layout.is_panel_enabled(DashboardPanelId::Battery));
}
