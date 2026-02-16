use std::sync::Arc;

use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::types::Config;

// Platform-specific power backend imports
#[cfg(target_os = "windows")]
use crate::cli::RyzenAdj as PowerBackend;

#[cfg(target_os = "linux")]
use crate::cli::LinuxPower as PowerBackend;

/// Power task: periodically reads config.power and applies via platform-specific backend
/// Strategy: poll every 2s and apply if profile changes or system state drifts
async fn tick(
    power_backend_lock: &Arc<tokio::sync::RwLock<Option<PowerBackend>>>,
    cfg: &Arc<tokio::sync::RwLock<Config>>,
    framework_tool_lock: &Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    last_profile_hash: &mut Option<u64>,
    last_apply_at: &mut Option<Instant>,
) {
    // Obtain current power backend from shared state; if missing, continue
    let Some(backend) = power_backend_lock.read().await.clone() else {
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
        // No profile configured, clear last hash
        *last_profile_hash = None;
        return;
    };

    // Simple hash of profile to detect changes
    let profile_hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();

        if let Some(tdp) = &profile.tdp_watts {
            tdp.enabled.hash(&mut hasher);
            tdp.value.hash(&mut hasher);
        }
        if let Some(thermal) = &profile.thermal_limit_c {
            thermal.enabled.hash(&mut hasher);
            thermal.value.hash(&mut hasher);
        }
        if let Some(epp) = &profile.epp_preference {
            epp.enabled.hash(&mut hasher);
            epp.value.hash(&mut hasher);
        }
        if let Some(gov) = &profile.governor {
            gov.enabled.hash(&mut hasher);
            gov.value.hash(&mut hasher);
        }
        if let Some(min) = &profile.min_freq_mhz {
            min.enabled.hash(&mut hasher);
            min.value.hash(&mut hasher);
        }
        if let Some(max) = &profile.max_freq_mhz {
            max.enabled.hash(&mut hasher);
            max.value.hash(&mut hasher);
        }

        hasher.finish()
    };

    let now = Instant::now();
    const REAPPLY_COOLDOWN_SECS: u64 = 2 * 60; // Reapply every 2 minutes to handle drift

    // Check if profile changed or cooldown elapsed
    let should_apply = match *last_profile_hash {
        None => true, // First time or after profile was cleared
        Some(prev_hash) => {
            if prev_hash != profile_hash {
                true // Profile changed
            } else {
                // Profile unchanged, check cooldown for drift protection
                match *last_apply_at {
                    None => true,
                    Some(t) => now.saturating_duration_since(t) >= Duration::from_secs(REAPPLY_COOLDOWN_SECS),
                }
            }
        }
    };

    if should_apply {
        info!("power: applying profile (ac={})", ac_present);
        if let Err(e) = backend.apply_profile(&profile).await {
            warn!("power: apply_profile failed: {}", e);
        } else {
            *last_profile_hash = Some(profile_hash);
            *last_apply_at = Some(now);
        }
    }
}

#[cfg(target_os = "windows")]
pub async fn run(
    power_backend_lock: Arc<tokio::sync::RwLock<Option<PowerBackend>>>,
    cfg: Arc<tokio::sync::RwLock<Config>>,
    framework_tool_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
) {
    info!("Power task started (Windows/RyzenAdj)");

    let mut last_profile_hash: Option<u64> = None;
    let mut last_apply_at: Option<Instant> = None;

    loop {
        tick(
            &power_backend_lock,
            &cfg,
            &framework_tool_lock,
            &mut last_profile_hash,
            &mut last_apply_at,
        )
        .await;

        sleep(Duration::from_secs(2)).await;
    }
}

#[cfg(target_os = "linux")]
pub async fn run(
    power_backend_lock: Arc<tokio::sync::RwLock<Option<PowerBackend>>>,
    cfg: Arc<tokio::sync::RwLock<Config>>,
    framework_tool_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
) {
    info!("Power task started (Linux native)");

    let mut last_profile_hash: Option<u64> = None;
    let mut last_apply_at: Option<Instant> = None;

    loop {
        tick(
            &power_backend_lock,
            &cfg,
            &framework_tool_lock,
            &mut last_profile_hash,
            &mut last_apply_at,
        )
        .await;

        sleep(Duration::from_secs(2)).await;
    }
}
