use std::sync::Arc;

use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

use crate::cli::{FrameworkTool, RyzenAdj};
use crate::types::Config;

/// Power task: periodically reads config.power and applies via RyzenAdj
/// Strategy: poll every 2s and apply if values differ from last applied
async fn tick(
    ryzenadj_lock: &Arc<tokio::sync::RwLock<Option<RyzenAdj>>>,
    cfg: &Arc<tokio::sync::RwLock<Config>>,
    framework_tool_lock: &Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    last_tdp: &mut Option<u32>,
    last_thermal: &mut Option<u32>,
    // Observed current TDP from ryzenadj --info
    observed_tdp: &mut Option<u32>,
    last_tdp_change_at: &mut Instant,
    last_tdp_apply_at: &mut Option<Instant>,
    last_info_poll_at: &mut Option<Instant>,
    observed_thermal: &mut Option<u32>,
    last_thermal_reapply_at: &mut Option<Instant>,
    startup_time: Instant,
) {
    // Obtain current RyzenAdj from shared state; if missing, continue
    let Some(ryz) = ryzenadj_lock.read().await.clone() else {
        return;
    };

    // Obtain framework_tool from shared state; if missing, continue
    let Some(ft) = framework_tool_lock.read().await.clone() else {
        return;
    };

    let cfg_power = { cfg.read().await.power.clone() };

    // Detect AC presence via framework_tool and continue if it fails
    let Ok(p) = ft.power().await else {
        return;
    };
    let Some(ac_present) = p.ac_present else {
        return;
    };

    // Select profile based on AC/battery state; only act if a profile exists
    let maybe_profile = if ac_present {
        cfg_power.ac
    } else {
        cfg_power.battery
    };
    let Some(profile) = maybe_profile else {
        return;
    };

    // Constants
    const TDP_TOLERANCE_W: u32 = 2; // within +/- 2W is OK
    const QUIET_WINDOW_SECS: u64 = 60; // wait for no drift for 60s
    const REAPPLY_COOLDOWN_SECS: u64 = 120; // do not reapply more often than every 120s
    const INFO_POLL_SECS: u64 = 5; // refresh current info every 5s
    const THERMAL_REAPPLY_COOLDOWN_SECS: u64 = 300; // retry thermal every 5 min if needed
    const THERMAL_BOOT_DELAY_SECS: u64 = 60; // wait 1 min before touching thermal

    let now = Instant::now();

    // Periodically poll current info from ryzenadj (cheap parse, external CLI)
    // We do this if we have a profile, regardless of specific enabled flags,
    // so we can observe state.
    let should_poll_info = match *last_info_poll_at {
        None => true,
        Some(t) => now.saturating_duration_since(t) >= Duration::from_secs(INFO_POLL_SECS),
    };
    if should_poll_info {
        if let Ok(info) = ryz.info().await {
            // Update observed TDP
            if let Some(cur) = info.tdp_watts {
                match *observed_tdp {
                    None => {
                        *observed_tdp = Some(cur);
                        *last_tdp_change_at = now;
                    }
                    Some(prev) => {
                        if prev.abs_diff(cur) >= 1 {
                            *observed_tdp = Some(cur);
                            *last_tdp_change_at = now;
                        }
                    }
                }
            }
            // Update observed Thermal
            if let Some(cur) = info.thermal_limit_c {
                *observed_thermal = Some(cur);
            }
        }
        *last_info_poll_at = Some(now);
    }

    // Apply TDP if enabled and value present (>0)
    if profile
        .tdp_watts
        .as_ref()
        .map(|s| s.enabled)
        .unwrap_or(false)
    {
        if let Some(target_watts) = profile
            .tdp_watts
            .as_ref()
            .map(|s| s.value)
            .filter(|&w| w > 0)
        {
            // Initial apply when target changes
            if *last_tdp != Some(target_watts) {
                debug!("power: applying tdp {}W", target_watts);
                if let Err(e) = ryz.set_tdp_watts(target_watts).await {
                    warn!("power: set_tdp_watts failed: {}", e);
                } else {
                    *last_tdp = Some(target_watts);
                    *last_tdp_apply_at = Some(now);
                }
            } else {
                // Sticky-but-patient reapply: if current drifted and has been quiet for a while, reapply with cooldown
                let current_opt = *observed_tdp;
                let diff_needs_reapply = current_opt
                    .map(|cur| cur.abs_diff(target_watts) > TDP_TOLERANCE_W)
                    .unwrap_or(false);
                let quiet_enough = now.saturating_duration_since(*last_tdp_change_at)
                    >= Duration::from_secs(QUIET_WINDOW_SECS);
                let past_cooldown = match *last_tdp_apply_at {
                    None => true,
                    Some(t) => {
                        now.saturating_duration_since(t)
                            >= Duration::from_secs(REAPPLY_COOLDOWN_SECS)
                    }
                };
                // let quiet_secs = now.saturating_duration_since(*last_tdp_change_at).as_secs();
                // let since_apply_secs =
                //     last_tdp_apply_at.map(|t| now.saturating_duration_since(t).as_secs());
                // debug!(
                //     "power: diff_needs_reapply={} quiet={}s since_apply={:?}s",
                //     diff_needs_reapply,
                //     quiet_secs,
                //     since_apply_secs,
                // );
                if diff_needs_reapply && quiet_enough && past_cooldown {
                    debug!(
                        "power: reapplying tdp {}W after quiet={}s diff={:?}",
                        target_watts,
                        now.saturating_duration_since(*last_tdp_change_at).as_secs(),
                        current_opt
                    );
                    if let Err(e) = ryz.set_tdp_watts(target_watts).await {
                        warn!("power: reapply set_tdp_watts failed: {}", e);
                    } else {
                        *last_tdp_apply_at = Some(now);
                    }
                }
            }
        }
    } else if last_tdp.is_some() {
        debug!("power: tdp disabled, skipping apply");
        *last_tdp = None;
    }

    // Apply thermal limit if enabled and value present (>0)
    if profile
        .thermal_limit_c
        .as_ref()
        .map(|s| s.enabled)
        .unwrap_or(false)
    {
        // Only apply thermal limit after boot delay has elapsed
        if now.saturating_duration_since(startup_time)
            >= Duration::from_secs(THERMAL_BOOT_DELAY_SECS)
        {
            if let Some(celsius) = profile
                .thermal_limit_c
                .as_ref()
                .map(|s| s.value)
                .filter(|&c| c > 0)
            {
                let target_changed = *last_thermal != Some(celsius);
                let drift = (*observed_thermal)
                    .map(|obs| obs != celsius)
                    .unwrap_or(false);
                let cooldown = match *last_thermal_reapply_at {
                    None => true,
                    Some(t) => {
                        now.saturating_duration_since(t)
                            >= Duration::from_secs(THERMAL_REAPPLY_COOLDOWN_SECS)
                    }
                };

                if target_changed || (drift && cooldown) {
                    debug!(
                        "power: applying thermal limit {}C (changed={} drift={} cooldown={})",
                        celsius, target_changed, drift, cooldown
                    );
                    if let Err(e) = ryz.set_thermal_limit_c(celsius).await {
                        warn!("power: set_thermal_limit_c failed: {}", e);
                    } else {
                        *last_thermal = Some(celsius);
                        *last_thermal_reapply_at = Some(now);
                    }
                }
            }
        }
    } else if last_thermal.is_some() {
        debug!("power: thermal disabled, skipping apply");
        *last_thermal = None;
    }
}

pub async fn run(
    ryzenadj_lock: Arc<tokio::sync::RwLock<Option<RyzenAdj>>>,
    cfg: Arc<tokio::sync::RwLock<Config>>,
    framework_tool_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
) {
    info!("Power task started");

    let mut last_tdp: Option<u32> = None;
    let mut last_thermal: Option<u32> = None;
    let mut observed_tdp: Option<u32> = None;
    let mut last_tdp_change_at: Instant = Instant::now();
    let mut last_tdp_apply_at: Option<Instant> = None;
    let mut last_info_poll_at: Option<Instant> = None;

    let mut observed_thermal: Option<u32> = None;
    let mut last_thermal_reapply_at: Option<Instant> = None;
    let startup_time = Instant::now();

    loop {
        tick(
            &ryzenadj_lock,
            &cfg,
            &framework_tool_lock,
            &mut last_tdp,
            &mut last_thermal,
            &mut observed_tdp,
            &mut last_tdp_change_at,
            &mut last_tdp_apply_at,
            &mut last_info_poll_at,
            &mut observed_thermal,
            &mut last_thermal_reapply_at,
            startup_time,
        )
        .await;

        sleep(Duration::from_secs(1)).await;
    }
}
