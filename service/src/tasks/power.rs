use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::cli::RyzenAdj;
use crate::types::Config;

/// Power task: periodically reads config.power and applies via RyzenAdj
/// Strategy: poll every 2s and apply if values differ from last applied
pub async fn run(ryzenadj: Option<RyzenAdj>, cfg: Arc<tokio::sync::RwLock<Config>>) {
    let Some(ryz) = ryzenadj else {
        warn!("power task: ryzenadj not available; skipping");
        return;
    };
    info!("Power task started");

    let mut last_tdp: Option<u32> = None;
    let mut last_thermal: Option<u32> = None;
    // Power profile removed

    loop {
        let cfg_power = { cfg.read().await.power.clone() };

        // Apply TDP if changed and present
        if let Some(watts) = cfg_power.tdp_watts {
            if last_tdp != Some(watts) {
                debug!("power: applying tdp {}W", watts);
                if let Err(e) = ryz.set_tdp_watts(watts).await {
                    warn!("power: set_tdp_watts failed: {}", e);
                } else {
                    last_tdp = Some(watts);
                }
            }
        }

        // Apply thermal limit if changed and present
        if let Some(celsius) = cfg_power.thermal_limit_c {
            if last_thermal != Some(celsius) {
                debug!("power: applying thermal limit {}C", celsius);
                if let Err(e) = ryz.set_thermal_limit_c(celsius).await {
                    warn!("power: set_thermal_limit_c failed: {}", e);
                } else {
                    last_thermal = Some(celsius);
                }
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
