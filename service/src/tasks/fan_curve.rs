use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::types::{Config, FanControlMode};

/// Main fan control task that runs continuously based on config
pub async fn run(cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>, cfg: Arc<tokio::sync::RwLock<Config>>) {
    info!("Fan control task started");

    let mut last_duty: Option<u32> = None;
    let mut last_mode: Option<FanControlMode> = None;
    let mut active_target: Option<u32> = None;
    let mut transition_start_temp: i32 = 0; // Used for hysteresis band

    loop {
        let loop_started = std::time::Instant::now();
        let config = cfg.read().await.fan.clone();
        // Loop cadence: use curve.poll_ms while in Curve mode with a curve present; otherwise a small fixed cadence
        let mode = config.mode.unwrap_or(FanControlMode::Disabled);
        let poll_interval = match (&mode, &config.curve) {
            (FanControlMode::Curve, Some(c)) => Duration::from_millis(c.poll_ms),
            _ => Duration::from_millis(500),
        };

        // Obtain current FrameworkTool from shared state; if missing, wait for next cadence and retry
        let maybe_cli = { cli_lock.read().await.clone() };
        let cli = match maybe_cli {
            Some(c) => c,
            None => {
                sleep(poll_interval).await;
                continue;
            }
        };

        // Handle based on current mode
        match &mode {
            // Disabled: let firmware handle it
            FanControlMode::Disabled => {
                if last_mode != Some(FanControlMode::Disabled) {
                    debug!("Mode change: {:?} -> Disabled", last_mode);
                    let _ = cli.autofanctrl().await;
                    last_duty = None;
                }
                last_mode = Some(FanControlMode::Disabled);
            }

            // Manual mode: set fixed duty
            FanControlMode::Manual => {
                if last_mode != Some(FanControlMode::Manual) {
                    debug!("Mode change: {:?} -> Manual", last_mode);
                }
                let target = if let Some(m) = &config.manual {
                    Some(m.duty_pct.min(100))
                } else {
                    None
                };
                if let Some(duty) = target {
                    let duty = duty.min(100);
                    if last_duty != Some(duty) {
                        debug!("Setting manual fan duty to {}%", duty);
                        if let Err(e) = cli.set_fan_duty(duty, None).await {
                            warn!("Failed to set fan duty: {}", e);
                        } else {
                            last_duty = Some(duty);
                            debug!("Manual: Set {}%", duty);
                        }
                    }
                } else {
                    // No manual duty set, fall back to auto
                    debug!("Manual: No duty set, switching to auto fan control");
                    let _ = cli.autofanctrl().await;
                    last_duty = None;
                }
                last_mode = Some(FanControlMode::Manual);
            }

            // Curve mode: dynamic control based on temperature
            FanControlMode::Curve => {
                if last_mode != Some(FanControlMode::Curve) {
                    debug!("Mode change: {:?} -> Curve", last_mode);
                }
                let Some(curve_cfg) = &config.curve else {
                    warn!("Curve mode without curve config; falling back to platform auto");
                    let _ = cli.autofanctrl().await;
                    last_duty = None;
                    sleep(poll_interval).await;
                    continue;
                };
                // 1. Read temperatures and select based on sensors (max across selection)
                let temp = get_max_sensor_temperature(&cli, &curve_cfg.sensors).await;
                let Some(temp) = temp else {
                    warn!("Failed to select temperature, continuing...");
                    sleep(poll_interval).await;
                    continue;
                };
                // If we just entered Curve mode, anchor hysteresis and clear target to avoid stale state
                if last_mode != Some(FanControlMode::Curve) {
                    debug!("Anchoring hysteresis at temp={}°C on entering Curve", temp);
                    transition_start_temp = temp;
                    active_target = None;
                }

                // 2. Compute instantaneous curve duty
                let curve_target = calculate_duty_from_curve(temp, &curve_cfg.points);

                // 3. Decide whether to accept this as the new active target
                match active_target {
                    None => {
                        active_target = Some(curve_target);
                        transition_start_temp = temp;
                    }
                    Some(current_target) if curve_target != current_target => {
                        if curve_target > current_target {
                            // Increasing – accept immediately
                            active_target = Some(curve_target);
                            transition_start_temp = temp;
                        } else {
                            // Decreasing – apply hysteresis with special handling:
                            // - If hysteresis is disabled, accept immediately
                            // - If temperature has increased since the transition anchor, accept immediately and re-anchor
                            // - Otherwise require temp to drop by hysteresis band
                            if curve_cfg.hysteresis_c == 0
                                || temp >= transition_start_temp
                                || temp <= transition_start_temp - curve_cfg.hysteresis_c as i32
                            {
                                active_target = Some(curve_target);
                                transition_start_temp = temp;
                            }
                        }
                    }
                    _ => {}
                }

                // 4. Step towards active_target every loop (rate-limited)
                if let Some(tgt) = active_target {
                    let mut decision = "hold";
                    let mut reason = "last==next";

                    let next = match last_duty {
                        Some(prev) if curve_cfg.rate_limit_pct_per_step < 100 => {
                            apply_rate_limit(prev, tgt, curve_cfg.rate_limit_pct_per_step)
                        }
                        _ => tgt,
                    };
                    if last_duty != Some(next) {
                        decision = "set";
                        reason = "advance";

                        if let Err(e) = cli.set_fan_duty(next, None).await {
                            warn!("Failed to set fan duty: {}", e);
                        } else {
                            last_duty = Some(next);
                        }
                    }

                    if decision != "hold" {
                        debug!(
                        "CurveLoop: temp={}°C, inst_target={}%, active_target={}%, anchor={}°C, hys={}°C, last_duty={:?}%, next={}%, step_limit={}%, decision={}, reason={}",
                        temp,
                        curve_target,
                        tgt,
                        transition_start_temp,
                        curve_cfg.hysteresis_c,
                        last_duty,
                        next,
                        curve_cfg.rate_limit_pct_per_step,
                            decision,
                            reason
                        );
                    }
                }
                last_mode = Some(FanControlMode::Curve);
            }
        }
        let elapsed = loop_started.elapsed();
        if elapsed < poll_interval {
            sleep(poll_interval - elapsed).await;
        }
    }
}

/// Read thermal and return the maximum temperature across the provided sensors.
async fn get_max_sensor_temperature(cli: &FrameworkTool, sensors: &[String]) -> Option<i32> {
    let output = cli.thermal().await.ok()?;
    let temps = &output.temps; // BTreeMap<String, i32>
    let mut best: Option<i32> = None;
    for name in sensors {
        if let Some(&v) = temps.get(name) {
            best = Some(match best { Some(b) => b.max(v), None => v });
            continue;
        }
        if let Some((_, v)) = temps.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            let v = *v;
            best = Some(match best { Some(b) => b.max(v), None => v });
        }
    }
    best
}

/// Calculate fan duty from temperature using the curve points
/// Always includes anchor points at [0,0] and [100,100] like the frontend
fn calculate_duty_from_curve(temp: i32, points: &[[u32; 2]]) -> u32 {
    let temp = temp as f64;

    // Build the full curve with anchor points, matching frontend behavior
    let mut full_curve = Vec::with_capacity(points.len() + 2);
    full_curve.push([0, 0]); // Start anchor
    full_curve.extend_from_slice(points);
    full_curve.push([100, 100]); // End anchor

    // Find the two points to interpolate between
    for window in full_curve.windows(2) {
        let [p1, p2] = window else { continue };
        let (x1, y1) = (p1[0] as f64, p1[1] as f64);
        let (x2, y2) = (p2[0] as f64, p2[1] as f64);

        if temp <= x1 {
            return y1 as u32; // Before first point
        }

        if temp <= x2 {
            // Linear interpolation between points
            if x2 == x1 {
                return y2 as u32;
            }
            let ratio = (temp - x1) / (x2 - x1);
            let duty = y1 + ratio * (y2 - y1);
            return duty.round() as u32;
        }
    }

    // Should never reach here due to [100,100] anchor, but just in case
    100
}

/// Apply rate limiting to duty changes
fn apply_rate_limit(current: u32, target: u32, max_change: u32) -> u32 {
    if target > current {
        current.saturating_add(max_change).min(target)
    } else {
        current.saturating_sub(max_change).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_duty_from_curve() {
        // Test with multiple points
        let points = [[40, 20], [60, 40], [75, 80]];

        // Test interpolation with anchor points
        assert_eq!(calculate_duty_from_curve(0, &points), 0); // Start anchor
        assert_eq!(calculate_duty_from_curve(20, &points), 10); // Between [0,0] and [40,20]
        assert_eq!(calculate_duty_from_curve(40, &points), 20); // Exact point
        assert_eq!(calculate_duty_from_curve(50, &points), 30); // Between [40,20] and [60,40]
        assert_eq!(calculate_duty_from_curve(60, &points), 40); // Exact point
        assert_eq!(calculate_duty_from_curve(75, &points), 80); // Exact point
        assert_eq!(calculate_duty_from_curve(87, &points), 88); // Between [75,80] and [100,100]
        assert_eq!(calculate_duty_from_curve(100, &points), 100); // End anchor

        // Test with empty points (just anchors)
        let empty: [[u32; 2]; 0] = [];
        assert_eq!(calculate_duty_from_curve(0, &empty), 0);
        assert_eq!(calculate_duty_from_curve(50, &empty), 50); // Linear from [0,0] to [100,100]
        assert_eq!(calculate_duty_from_curve(75, &empty), 75);
        assert_eq!(calculate_duty_from_curve(100, &empty), 100);

        // Test with single point
        let single = [[50, 30]];
        assert_eq!(calculate_duty_from_curve(0, &single), 0); // Start anchor
        assert_eq!(calculate_duty_from_curve(25, &single), 15); // Between [0,0] and [50,30]
        assert_eq!(calculate_duty_from_curve(50, &single), 30); // Exact point
        assert_eq!(calculate_duty_from_curve(75, &single), 65); // Between [50,30] and [100,100]
        assert_eq!(calculate_duty_from_curve(100, &single), 100); // End anchor
    }

    #[test]
    fn test_apply_rate_limit() {
        // Test increasing
        assert_eq!(apply_rate_limit(30, 50, 10), 40);
        assert_eq!(apply_rate_limit(30, 35, 10), 35);

        // Test decreasing
        assert_eq!(apply_rate_limit(50, 30, 10), 40);
        assert_eq!(apply_rate_limit(50, 45, 10), 45);

        // Test no limit (100%)
        assert_eq!(apply_rate_limit(30, 80, 100), 80);
    }
}
