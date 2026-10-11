use std::collections::HashMap;
use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::config::LiveConfig;
use crate::types::{CurveConfig, DashboardPanelId, FanControlMode};

/// Main fan control task that runs continuously based on config
pub async fn run(cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>, cfg: LiveConfig) {
    info!("Fan control task started");

    let mut global = CurveStepper::new();
    let mut per_fan_curve_steppers: HashMap<u32, CurveStepper> = HashMap::new();
    let mut last_manual_duty: HashMap<Option<u32>, u32> = HashMap::new();

    let mut last_mode: Option<FanControlMode> = None;
    let mut last_per_fan_active = false;
    let mut fan_count: Option<u32> = None;

    loop {
        let loop_started = std::time::Instant::now();
        let (config, fan_enabled) = {
            let cfg = cfg.read().await;
            (cfg.fan.clone(), cfg.ui.is_panel_enabled(DashboardPanelId::Fan))
        };
        let mode = config.mode.unwrap_or(FanControlMode::Disabled);

        let overrides = config.overrides.clone().unwrap_or_default();
        let per_fan_active = !overrides.is_empty();

        let poll_interval = Duration::from_millis(match mode {
            FanControlMode::Curve => config.curve.as_ref().map(|c| c.poll_ms).unwrap_or(500),
            _ => 500,
        });

        if !fan_enabled {
            if last_mode.take().is_some() {
                global.reset();
                per_fan_curve_steppers.clear();
                last_manual_duty.clear();
                last_per_fan_active = false;
                fan_count = None;
                if let Some(cli) = { cli_lock.read().await.clone() } {
                    let _ = cli.autofanctrl().await;
                }
            }
            sleep(poll_interval).await;
            continue;
        }

        // Obtain current FrameworkTool from shared state; if missing, reset and retry.
        let maybe_cli = { cli_lock.read().await.clone() };
        let cli = match maybe_cli {
            Some(c) => c,
            None => {
                global.reset();
                per_fan_curve_steppers.clear();
                last_manual_duty.clear();
                fan_count = None;
                sleep(poll_interval).await;
                continue;
            }
        };

        // Reset transient control state when the mode or the global/per-fan
        // topology changes so we re-anchor cleanly.
        if last_mode != Some(mode.clone()) || last_per_fan_active != per_fan_active {
            debug!(
                "Fan state change: mode {:?} -> {:?}, per_fan {} -> {}",
                last_mode, mode, last_per_fan_active, per_fan_active
            );
            global.reset();
            per_fan_curve_steppers.clear();
            last_manual_duty.clear();
        }

        match &mode {
            FanControlMode::Disabled => {
                if last_mode != Some(FanControlMode::Disabled) {
                    let _ = cli.autofanctrl().await;
                }
            }

            FanControlMode::Manual => {
                let global_duty = config.manual.as_ref().map(|m| m.duty_pct.min(100));
                if per_fan_active {
                    let Some(count) = ensure_fan_count(&cli, &mut fan_count).await else {
                        sleep(poll_interval).await;
                        continue;
                    };
                    for i in 0..count {
                        let duty = overrides
                            .iter()
                            .find(|o| o.index == i)
                            .and_then(|o| o.manual.as_ref())
                            .map(|m| m.duty_pct.min(100))
                            .or(global_duty);
                        if let Some(duty) = duty {
                            apply_manual(&cli, &mut last_manual_duty, Some(i), duty).await;
                        }
                    }
                } else if let Some(duty) = global_duty {
                    apply_manual(&cli, &mut last_manual_duty, None, duty).await;
                } else {
                    // No manual duty configured: fall back to firmware auto.
                    let _ = cli.autofanctrl().await;
                }
            }

            FanControlMode::Curve => {
                if per_fan_active {
                    let Some(count) = ensure_fan_count(&cli, &mut fan_count).await else {
                        sleep(poll_interval).await;
                        continue;
                    };
                    for i in 0..count {
                        let curve = overrides
                            .iter()
                            .find(|o| o.index == i)
                            .and_then(|o| o.curve.clone())
                            .or_else(|| config.curve.as_ref().map(|c| c.curve.clone()));
                        let Some(curve) = curve else { continue };
                        let stepper = per_fan_curve_steppers.entry(i).or_insert_with(CurveStepper::new);
                        apply_curve(&cli, stepper, &curve, Some(i)).await;
                    }
                } else {
                    let Some(curve) = config.curve.as_ref().map(|c| c.curve.clone()) else {
                        warn!("Curve mode without curve config; falling back to platform auto");
                        let _ = cli.autofanctrl().await;
                        sleep(poll_interval).await;
                        continue;
                    };
                    apply_curve(&cli, &mut global, &curve, None).await;
                }
            }
        }

        last_mode = Some(mode);
        last_per_fan_active = per_fan_active;

        let elapsed = loop_started.elapsed();
        if elapsed < poll_interval {
            sleep(poll_interval - elapsed).await;
        }
    }
}

/// Encapsulates the hysteresis + rate-limit state machine for a single fan.
struct CurveStepper {
    last_duty: Option<u32>,
    active_target: Option<u32>,
    transition_start_temp: i32,
}

impl CurveStepper {
    fn new() -> Self {
        Self {
            last_duty: None,
            active_target: None,
            transition_start_temp: 0,
        }
    }

    fn reset(&mut self) {
        self.last_duty = None;
        self.active_target = None;
    }

    fn note_applied(&mut self, duty: u32) {
        self.last_duty = Some(duty);
    }

    /// Advance the state machine for the given temperature and return the duty to apply, or `None` when the current duty should be held.
    fn next(&mut self, temp: i32, curve: &CurveConfig) -> Option<u32> {
        let curve_target = calculate_duty_from_curve(temp, &curve.points);

        match self.active_target {
            // First evaluation after a reset anchors the hysteresis band.
            None => {
                self.active_target = Some(curve_target);
                self.transition_start_temp = temp;
            }
            Some(current) if curve_target != current => {
                if curve_target > current {
                    // Increasing – accept immediately.
                    self.active_target = Some(curve_target);
                    self.transition_start_temp = temp;
                } else if curve.hysteresis_c == 0
                    || temp >= self.transition_start_temp
                    || temp <= self.transition_start_temp - curve.hysteresis_c as i32
                {
                    // Decreasing – accept once outside the hysteresis band (or
                    // immediately if hysteresis is disabled / temp has risen).
                    self.active_target = Some(curve_target);
                    self.transition_start_temp = temp;
                }
            }
            _ => {}
        }

        let tgt = self.active_target?;
        let next = match self.last_duty {
            Some(prev) => {
                // Spin-up uses rate_limit_pct_per_step; spin-down uses the
                // optional down rate, falling back to the up rate when unset.
                let rate = if tgt >= prev {
                    curve.rate_limit_pct_per_step
                } else {
                    curve
                        .rate_limit_down_pct_per_step
                        .unwrap_or(curve.rate_limit_pct_per_step)
                };
                apply_rate_limit(prev, tgt, rate)
            }
            None => tgt,
        };

        if self.last_duty != Some(next) {
            Some(next)
        } else {
            None
        }
    }
}

/// Evaluate a curve for one fan and apply the resulting duty (if it changed).
async fn apply_curve(cli: &FrameworkTool, stepper: &mut CurveStepper, curve: &CurveConfig, fan_index: Option<u32>) {
    let Some(temp) = get_max_sensor_temperature(cli, &curve.sensors).await else {
        warn!("Failed to select temperature for fan {:?}, continuing...", fan_index);
        return;
    };
    if let Some(next) = stepper.next(temp, curve) {
        match cli.set_fan_duty(next, fan_index).await {
            Ok(()) => {
                stepper.note_applied(next);
                debug!("Curve: fan {:?} -> {}% at {}°C", fan_index, next, temp);
            }
            Err(e) => warn!("Failed to set fan {:?} duty: {}", fan_index, e),
        }
    }
}

/// Apply a manual duty for one fan, skipping redundant CLI calls.
async fn apply_manual(
    cli: &FrameworkTool,
    last_manual_duty: &mut HashMap<Option<u32>, u32>,
    fan_index: Option<u32>,
    duty: u32,
) {
    if last_manual_duty.get(&fan_index) == Some(&duty) {
        return;
    }
    match cli.set_fan_duty(duty, fan_index).await {
        Ok(()) => {
            last_manual_duty.insert(fan_index, duty);
            debug!("Manual: fan {:?} -> {}%", fan_index, duty);
        }
        Err(e) => warn!("Failed to set fan {:?} duty: {}", fan_index, e),
    }
}

/// Cached fan-count lookup: detect once, then reuse until the caller resets it.
async fn ensure_fan_count(cli: &FrameworkTool, cached: &mut Option<u32>) -> Option<u32> {
    if let Some(c) = *cached {
        return Some(c);
    }
    let count = cli.thermal().await.ok()?.fans.len() as u32;
    // Don't cache a zero reading (thermal not ready yet); retry next tick.
    if count == 0 {
        return None;
    }
    *cached = Some(count);
    Some(count)
}

/// Read thermal and return the maximum temperature across the provided sensors.
async fn get_max_sensor_temperature(cli: &FrameworkTool, sensors: &[String]) -> Option<i32> {
    let output = cli.thermal().await.ok()?;
    max_sensor_temperature(&output.temps, sensors)
}

fn max_sensor_temperature(temps: &std::collections::BTreeMap<String, i32>, sensors: &[String]) -> Option<i32> {
    let mut best: Option<i32> = None;
    for name in sensors {
        if let Some(&v) = temps.get(name) {
            best = Some(match best {
                Some(b) => b.max(v),
                None => v,
            });
            continue;
        }
        if let Some((_, v)) = temps.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)) {
            let v = *v;
            best = Some(match best {
                Some(b) => b.max(v),
                None => v,
            });
        }
    }
    // Sensor names changed in newer framework_tool/EC versions; don't leave the fan stuck on stale names.
    best.or_else(|| temps.values().copied().max())
}

/// Calculate fan duty from temperature using the curve points
/// Always includes anchor points at [0,0] and [100,100] like the frontend
fn calculate_duty_from_curve(temp: i32, points: &[[u32; 2]]) -> u32 {
    let temp = temp as f64;

    let mut full_curve = Vec::with_capacity(points.len() + 2);
    full_curve.push([0, 0]);
    full_curve.extend_from_slice(points);
    full_curve.push([100, 100]);
    // Config may hold unsorted points (API/hand edits); the UI draws them sorted.
    full_curve.sort_by_key(|p| p[0]);

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
            let ratio = (temp - x1) / (x2 - x1);
            let duty = y1 + ratio * (y2 - y1);
            return duty.round() as u32;
        }
    }

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
#[path = "fan_curve_tests.rs"]
mod tests;
