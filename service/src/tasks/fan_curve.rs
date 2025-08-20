use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{debug, info, warn};

use crate::cli::FrameworkTool;
use crate::types::{Config, FanMode};

/// Main fan control task that runs continuously based on config
pub async fn run(cli: FrameworkTool, cfg: Arc<tokio::sync::RwLock<Config>>) {
    info!("Fan control task started");
    
    let mut last_duty: Option<u32> = None;
    let mut last_change_temp: Option<i32> = None;  // Temperature at last duty change
    let mut last_config_hash: Option<u64> = None;  // Track config changes

    loop {
        let config = cfg.read().await.fan_curve.clone();
        let poll_interval = Duration::from_millis(config.poll_ms.max(200));
        
        // Calculate a simple hash to detect config changes
        let config_hash = calculate_config_hash(&config);
        let config_changed = last_config_hash != Some(config_hash);
        
        if config_changed {
            debug!("Fan config changed, applying immediately");
            // Reset state to force immediate update
            last_duty = None;
            last_change_temp = None;
            last_config_hash = Some(config_hash);
        }

        // Handle based on current mode
        match (config.enabled, &config.mode) {
            // Disabled or Auto mode: let firmware handle it
            (false, _) | (_, FanMode::Auto) => {
                if last_duty.is_some() {
                    debug!("Switching to auto fan control");
            let _ = cli.autofanctrl().await;
                    last_duty = None;
                    last_change_temp = None;
                }
                sleep(poll_interval).await;
            }
            
            // Manual mode: set fixed duty
            (true, FanMode::Manual) => {
                if let Some(duty) = config.manual_duty_pct {
                    let duty = duty.min(100);
                    if last_duty != Some(duty) {
                        debug!("Setting manual fan duty to {}%", duty);
                        if let Err(e) = cli.set_fan_duty(duty, None).await {
                            warn!("Failed to set fan duty: {}", e);
                        } else {
                        last_duty = Some(duty);
                    }
                    }
                } else {
                    // No manual duty set, fall back to auto
                    let _ = cli.autofanctrl().await;
                    last_duty = None;
                }
                sleep(poll_interval).await;
        }

            // Curve mode: dynamic control based on temperature
            (true, FanMode::Curve) => {
                // Read current temperature
                let temp = match get_sensor_temperature(&cli, &config.sensor).await {
            Some(t) => t,
                    None => {
                        warn!("Failed to read temperature, continuing...");
                        sleep(poll_interval).await;
                        continue;
                    }
                };

                // Calculate target duty from curve
                let target_duty = calculate_duty_from_curve(temp, &config.points);
                
                // Apply hysteresis only when decreasing (to prevent oscillation when cooling)
                let should_update = match (last_duty, last_change_temp) {
                    (Some(prev_duty), Some(prev_temp)) if config.hysteresis_c > 0 => {
                        if target_duty > prev_duty {
                            // Increasing duty: always respond immediately for better cooling
                            true
                        } else if target_duty < prev_duty {
                            // Decreasing duty: apply hysteresis to prevent oscillation
                            temp <= prev_temp - config.hysteresis_c as i32
                        } else {
                            // Same duty, no change needed
                            false
                        }
                    }
                    (Some(prev_duty), _) => {
                        // No hysteresis configured, update if duty would change
                        target_duty != prev_duty
                    }
                    _ => true, // Always update on first run or after mode change
                };

                if should_update {
                    // Apply rate limiting if configured
                    let next_duty = match last_duty {
                        Some(prev) if config.rate_limit_pct_per_step < 100 => {
                            apply_rate_limit(prev, target_duty, config.rate_limit_pct_per_step)
                        }
                        _ => target_duty,
                    };

                    debug!("Temp: {}°C, Target: {}%, Setting: {}%", temp, target_duty, next_duty);
                    
                    if let Err(e) = cli.set_fan_duty(next_duty, None).await {
                        warn!("Failed to set fan duty: {}", e);
                    } else {
                        last_duty = Some(next_duty);
                        last_change_temp = Some(temp);  // Record temperature at change
                    }
                }
                
                sleep(poll_interval).await;
            }
        }
    }
}

/// Get temperature from thermal sensors
async fn get_sensor_temperature(cli: &FrameworkTool, sensor: &str) -> Option<i32> {
    match cli.thermal().await {
        Ok(output) => parse_temperature(&output, sensor),
        Err(e) => {
            debug!("Failed to read thermal data: {}", e);
            None
        }
    }
}

/// Parse temperature from thermal output
/// Looks for lines like "APU:    62 C" or "CPU:    55 C"
fn parse_temperature(output: &str, sensor: &str) -> Option<i32> {
    // First try to find the specific sensor
    let sensor_prefix = format!("{}:", sensor);
    
    for line in output.lines() {
        let trimmed = line.trim();
        
        // Check if this line contains our sensor
        if trimmed.contains(&sensor_prefix) {
            // Extract temperature value (looking for pattern: "number C")
            if let Some(c_pos) = trimmed.rfind(" C") {
                // Get the substring before " C"
                let before_c = &trimmed[..c_pos];
                // Extract the last word (should be the temperature number)
                if let Some(temp_str) = before_c.split_whitespace().last() {
                    if let Ok(temp) = temp_str.parse::<i32>() {
                        return Some(temp);
                    }
                }
            }
        }
    }
    
    // Fallback: try "APU" if original sensor not found
    if sensor != "APU" {
        return parse_temperature(output, "APU");
    }
    
    None
}

/// Calculate fan duty from temperature using the curve points
/// Always includes anchor points at [0,0] and [100,100] like the frontend
fn calculate_duty_from_curve(temp: i32, points: &[[u32; 2]]) -> u32 {
    let temp = temp as f64;
    
    // Build the full curve with anchor points, matching frontend behavior
    let mut full_curve = Vec::with_capacity(points.len() + 2);
    full_curve.push([0, 0]);  // Start anchor
    full_curve.extend_from_slice(points);
    full_curve.push([100, 100]);  // End anchor
    
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

/// Calculate a hash of config to detect changes
fn calculate_config_hash(config: &crate::types::FanCurveConfig) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    
    // Hash all relevant config fields
    config.enabled.hash(&mut hasher);
    format!("{:?}", config.mode).hash(&mut hasher);
    config.sensor.hash(&mut hasher);
    
    // Hash the curve points
    for point in &config.points {
        point[0].hash(&mut hasher);
        point[1].hash(&mut hasher);
    }
    
    config.hysteresis_c.hash(&mut hasher);
    config.rate_limit_pct_per_step.hash(&mut hasher);
    config.manual_duty_pct.hash(&mut hasher);
    // Note: We don't hash poll_ms as changing polling interval shouldn't reset the curve
    
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_temperature() {
        let output = "  F75303_Local:   45 C\n  F75303_CPU:     55 C\n  APU:          62 C\n";
        
        assert_eq!(parse_temperature(output, "APU"), Some(62));
        assert_eq!(parse_temperature(output, "F75303_CPU"), Some(55));
        assert_eq!(parse_temperature(output, "CPU"), None); // Exact match required
    }

    #[test]
    fn test_calculate_duty_from_curve() {
        // Test with multiple points
        let points = [
            [40, 20],
            [60, 40],
            [75, 80],
        ];
        
        // Test interpolation with anchor points
        assert_eq!(calculate_duty_from_curve(0, &points), 0);    // Start anchor
        assert_eq!(calculate_duty_from_curve(20, &points), 10);  // Between [0,0] and [40,20]
        assert_eq!(calculate_duty_from_curve(40, &points), 20);  // Exact point
        assert_eq!(calculate_duty_from_curve(50, &points), 30);  // Between [40,20] and [60,40]
        assert_eq!(calculate_duty_from_curve(60, &points), 40);  // Exact point
        assert_eq!(calculate_duty_from_curve(75, &points), 80);  // Exact point
        assert_eq!(calculate_duty_from_curve(87, &points), 88);  // Between [75,80] and [100,100]
        assert_eq!(calculate_duty_from_curve(100, &points), 100); // End anchor
        
        // Test with empty points (just anchors)
        let empty: [[u32; 2]; 0] = [];
        assert_eq!(calculate_duty_from_curve(0, &empty), 0);
        assert_eq!(calculate_duty_from_curve(50, &empty), 50);  // Linear from [0,0] to [100,100]
        assert_eq!(calculate_duty_from_curve(75, &empty), 75);
        assert_eq!(calculate_duty_from_curve(100, &empty), 100);
        
        // Test with single point
        let single = [[50, 30]];
        assert_eq!(calculate_duty_from_curve(0, &single), 0);    // Start anchor
        assert_eq!(calculate_duty_from_curve(25, &single), 15);  // Between [0,0] and [50,30]
        assert_eq!(calculate_duty_from_curve(50, &single), 30);  // Exact point
        assert_eq!(calculate_duty_from_curve(75, &single), 65);  // Between [50,30] and [100,100]
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