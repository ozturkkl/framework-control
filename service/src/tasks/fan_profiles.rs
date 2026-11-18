use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::cli::FrameworkTool;
use crate::config::LiveConfig;
use crate::types::{Config, DashboardPanelId, FanProfileAutoSwitch};

const POLL_SECS: u64 = 2;

/// Switches the active fan profile when the power source or its assigned profile changes.
pub async fn run(cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>, cfg: LiveConfig) {
    info!("Fan profile auto-switch task started");
    let mut applied: Option<(bool, Option<String>)> = None;

    loop {
        sleep(Duration::from_secs(POLL_SECS)).await;

        let (auto, fan_enabled) = {
            let cfg = cfg.read().await;
            (
                cfg.fan.auto_switch.clone(),
                cfg.ui.is_panel_enabled(DashboardPanelId::Fan),
            )
        };
        if !fan_enabled || auto == FanProfileAutoSwitch::default() {
            applied = None;
            continue;
        }
        let Some(cli) = cli_lock.read().await.clone() else {
            continue;
        };
        let Ok(power) = cli.power().await else {
            continue;
        };
        // The UI hides auto-switch without a battery, so a stale setting must not apply.
        if power.battery_present == Some(false) {
            continue;
        }
        let Some(ac_present) = power.ac_present else {
            continue;
        };
        let target = if ac_present { auto.ac } else { auto.battery };
        let current = Some((ac_present, target.clone()));
        if applied == current {
            continue;
        }
        applied = current;
        let Some(id) = target else {
            continue;
        };

        let name = {
            let cfg = cfg.read().await;
            if cfg.fan.active_profile == id {
                continue;
            }
            cfg.fan.profiles.iter().find(|p| p.id == id).map(|p| p.name.clone())
        };
        info!(
            "fan profiles: {} -> switching to '{}'",
            if ac_present { "plugged in" } else { "on battery" },
            name.as_deref().unwrap_or(&id)
        );
        let switch = |c: &mut Config| {
            c.fan.active_profile = id.clone();
            c.fan.validate()
        };
        if let Err(e) = cfg.persist(None, switch).await {
            warn!("fan profiles: failed to persist switch: {}", e);
        }
    }
}
