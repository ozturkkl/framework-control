use std::sync::Arc;

use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::types::{BatteryConfig, Config};

/// Battery task: applies config.battery settings when they change and periodically every 30 minutes.
pub async fn run(
    framework_tool_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    cfg: Arc<tokio::sync::RwLock<Config>>,
) {
    info!("Battery task started");

    const REAPPLY_INTERVAL_SECS: u64 = 30 * 60;
    const CL_MIN: u8 = 25;
    const CL_MAX: u8 = 100;

    let mut last_charge_limit_pct: Option<u8> = None;
    let mut last_rate_c: Option<f32> = None;
    let mut last_threshold_pct: Option<u8> = None;
    let mut last_charge_apply_at: Option<Instant> = None;
    let mut last_rate_apply_at: Option<Instant> = None;

    loop {
        // Clone required shared state each tick
        let cfg_bat: BatteryConfig = { cfg.read().await.battery.clone() };
        let ft_opt = { framework_tool_lock.read().await.clone() };

        if let Some(cli) = ft_opt {
            // Apply charge limit
            if let Some(setting) = cfg_bat.charge_limit_max_pct.clone() {
                // If disabled, restore to 100%
                let desired = if setting.enabled {
                    setting
                        .value
                        .clamp(CL_MIN, CL_MAX)
                } else {
                    100
                };
                let need_apply = match last_charge_limit_pct {
                    None => true,
                    Some(prev) => prev != desired,
                };
                let past_reapply = match last_charge_apply_at {
                    None => true,
                    Some(t) => Instant::now().saturating_duration_since(t)
                        >= Duration::from_secs(REAPPLY_INTERVAL_SECS),
                };
                if need_apply || past_reapply {
                    debug!("battery: applying charge limit {}%", desired);
                    match cli.charge_limit_set(desired).await {
                        Ok(_) => {
                            last_charge_limit_pct = Some(desired);
                            last_charge_apply_at = Some(Instant::now());
                        }
                        Err(e) => {
                            warn!("battery: charge_limit_set failed: {}", e);
                        }
                    }
                }
            }

            // Apply rate limit (C). Disabled approximates "no limit" by using 1.0C.
            if let Some(setting) = cfg_bat.charge_rate_c.clone() {
                let mut desired_c = if setting.enabled { setting.value } else { 1.0 };
                // snap to 0.05 steps like UI
                desired_c = (desired_c * 20.0).round() / 20.0;
                desired_c = desired_c.clamp(0.0, 1.0);
                let desired_threshold = cfg_bat.charge_rate_soc_threshold_pct;
                let need_apply = match (last_rate_c, last_threshold_pct) {
                    (Some(prev_c), prev_t) => prev_c != desired_c || prev_t != desired_threshold,
                    _ => true,
                };
                let past_reapply = match last_rate_apply_at {
                    None => true,
                    Some(t) => Instant::now().saturating_duration_since(t)
                        >= Duration::from_secs(REAPPLY_INTERVAL_SECS),
                };
                if need_apply || past_reapply {
                    debug!(
                        "battery: applying charge rate {}C (threshold={:?})",
                        desired_c, desired_threshold
                    );
                    match cli
                        .charge_rate_limit_set(desired_c, desired_threshold)
                        .await
                    {
                        Ok(_) => {
                            last_rate_c = Some(desired_c);
                            last_threshold_pct = desired_threshold;
                            last_rate_apply_at = Some(Instant::now());
                        }
                        Err(e) => {
                            warn!("battery: charge_rate_limit_set failed: {}", e);
                        }
                    }
                }
            }
        }

        sleep(Duration::from_secs(1)).await;
    }
}


