use std::collections::HashMap;
use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::types::{Config, CurveConfig, FanControlMode};

/// Main fan control task that runs continuously based on config
pub async fn run(cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>, cfg: Arc<tokio::sync::RwLock<Config>>) {
    info!("Fan control task started");

    let mut global = CurveStepper::new();
    let mut per_fan_curve_steppers: HashMap<u32, CurveStepper> = HashMap::new();
    let mut last_manual_duty: HashMap<Option<u32>, u32> = HashMap::new();

    let mut last_mode: Option<FanControlMode> = None;
    let mut last_per_fan_active = false;
    let mut fan_count: Option<u32> = None;

    loop {
        let loop_started = std::time::Instant::now();
        let config = cfg.read().await.fan.clone();
        let mode = config.mode.unwrap_or(FanControlMode::Disabled);

        let overrides = config.overrides.clone().unwrap_or_default();
        let per_fan_active = !overrides.is_empty();

        let poll_interval = Duration::from_millis(match mode {
            FanControlMode::Curve => config.curve.as_ref().map(|c| c.poll_ms).unwrap_or(500),
            _ => 500,
        });

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
                            .or_else(|| config.curve.clone());
                        let Some(curve) = curve else { continue };
                        let stepper = per_fan_curve_steppers.entry(i).or_insert_with(CurveStepper::new);
                        apply_curve(&cli, stepper, &curve, Some(i)).await;
                    }
                } else {
                    let Some(curve) = &config.curve else {
                        warn!("Curve mode without curve config; falling back to platform auto");
                        let _ = cli.autofanctrl().await;
                        sleep(poll_interval).await;
                        continue;
                    };
                    apply_curve(&cli, &mut global, curve, None).await;
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
    anchored: bool,
}

impl CurveStepper {
    fn new() -> Self {
        Self {
            last_duty: None,
            active_target: None,
            transition_start_temp: 0,
            anchored: false,
        }
    }

    fn reset(&mut self) {
        self.last_duty = None;
        self.active_target = None;
        self.anchored = false;
    }

    fn note_applied(&mut self, duty: u32) {
        self.last_duty = Some(duty);
    }

    /// Advance the state machine for the given temperature and return the duty to apply, or `None` when the current duty should be held.
    fn next(&mut self, temp: i32, curve: &CurveConfig) -> Option<u32> {
        // Anchor hysteresis on first evaluation after a reset.
        if !self.anchored {
            self.transition_start_temp = temp;
            self.active_target = None;
            self.anchored = true;
        }

        let curve_target = calculate_duty_from_curve(temp, &curve.points);

        match self.active_target {
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
    let temps = &output.temps;
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
    best
}

/// Calculate fan duty from temperature using the curve points
/// Always includes anchor points at [0,0] and [100,100] like the frontend
fn calculate_duty_from_curve(temp: i32, points: &[[u32; 2]]) -> u32 {
    let temp = temp as f64;

    let mut full_curve = Vec::with_capacity(points.len() + 2);
    full_curve.push([0, 0]);
    full_curve.extend_from_slice(points);
    full_curve.push([100, 100]);

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
        assert_eq!(calculate_duty_from_curve(87, &points), 90); // Between [75,80] and [100,100]
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

    fn curve(points: Vec<[u32; 2]>, hysteresis_c: u32, rate_limit_pct_per_step: u32) -> CurveConfig {
        CurveConfig {
            sensors: vec![],
            points,
            poll_ms: 2000,
            hysteresis_c,
            rate_limit_pct_per_step,
            rate_limit_down_pct_per_step: None,
        }
    }

    #[test]
    fn stepper_anchors_then_holds() {
        let c = curve(vec![[40, 20], [60, 40]], 0, 100);
        let mut s = CurveStepper::new();
        // First evaluation applies the computed duty.
        let first = s.next(50, &c);
        assert_eq!(first, Some(30));
        s.note_applied(30);
        // Same temperature holds (no redundant apply).
        assert_eq!(s.next(50, &c), None);
    }

    #[test]
    fn stepper_hysteresis_blocks_small_drops() {
        let c = curve(vec![[40, 20], [60, 40]], 5, 100);
        let mut s = CurveStepper::new();
        s.note_applied(40);
        // Anchor at 60 -> target 40, already applied so holds.
        assert_eq!(s.next(60, &c), None);
        // Drop of 2°C is inside the 5°C band -> still holds at 40.
        assert_eq!(s.next(58, &c), None);
        // Drop beyond the band -> accept the lower target.
        let dropped = s.next(54, &c);
        assert_eq!(dropped, Some(34));
    }

    #[test]
    fn stepper_rate_limit_steps_toward_target() {
        let c = curve(vec![[40, 20], [60, 80]], 0, 10);
        let mut s = CurveStepper::new();
        s.note_applied(20);
        // Target at 60°C is 80, but rate limit caps the step at +10.
        assert_eq!(s.next(60, &c), Some(30));
    }

    #[test]
    fn stepper_separate_down_rate_limit() {
        // Fast spin-up (100 = instant), slow spin-down (5% per step).
        let mut c = curve(vec![[40, 20], [60, 80]], 0, 100);
        c.rate_limit_down_pct_per_step = Some(5);
        let mut s = CurveStepper::new();
        s.note_applied(80);
        // Target at 40°C is 20; the down rate caps the drop at -5 -> 75.
        assert_eq!(s.next(40, &c), Some(75));
    }
}
