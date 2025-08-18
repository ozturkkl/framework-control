use std::sync::Arc;

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::cli::FrameworkTool;
use crate::types::{Config, FanMode};

pub async fn run(cli: FrameworkTool, cfg: Arc<tokio::sync::RwLock<Config>>) {
    info!("Fan curve task starting");
    let mut last_duty: Option<u32> = None;
    let mut last_temp: Option<i32> = None;
    let mut last_mode: Option<FanMode> = None;
    let mut last_enabled: Option<bool> = None;

    loop {
        let snapshot = cfg.read().await.fan_curve.clone();
        if Some(snapshot.enabled) != last_enabled || Some(snapshot.mode.clone()) != last_mode {
            // Reset state when enabled/mode changes
            last_duty = None;
            last_temp = None;
            last_mode = Some(snapshot.mode.clone());
            last_enabled = Some(snapshot.enabled);
        }

        if !snapshot.enabled {
            // Ensure platform fan control is in auto when our control is disabled
            let _ = cli.autofanctrl().await;
            sleep(Duration::from_millis(1000)).await;
            continue;
        }

        // Mode handling
        match snapshot.mode {
            FanMode::Auto => {
                let _ = cli.autofanctrl().await;
                last_duty = None;
                last_temp = None;
                sleep(Duration::from_millis(snapshot.poll_ms)).await;
                continue;
            }
            FanMode::Manual => {
                if let Some(duty) = snapshot.manual_duty_pct {
                    let duty = duty.min(100);
                    if last_duty != Some(duty) {
                        let _ = cli.set_fan_duty(duty, None).await;
                        last_duty = Some(duty);
                    }
                    sleep(Duration::from_millis(snapshot.poll_ms)).await;
                    continue;
                } else {
                    // fallback to auto if manual without value
                    let _ = cli.autofanctrl().await;
                    sleep(Duration::from_millis(snapshot.poll_ms)).await;
                    continue;
                }
            }
            FanMode::Curve => {}
        }

        let temp_c = match read_temperature(&cli, &snapshot.sensor).await {
            Some(t) => t,
            None => { sleep(Duration::from_millis(snapshot.poll_ms)).await; continue; }
        };

        let target = map_temp_to_duty(temp_c, &snapshot.points);

        let should_change = match (last_duty, last_temp) {
            (Some(prev), Some(prev_t)) => {
                let temp_delta = (temp_c - prev_t).abs() as u32;
                let duty_delta = if target > prev { target - prev } else { prev - target };
                temp_delta >= snapshot.hysteresis_c || duty_delta >= 1
            }
            _ => true,
        };

        if should_change {
            let next = if let Some(prev) = last_duty {
                apply_rate_limit(prev, target, snapshot.rate_limit_pct_per_step)
            } else { target };

            match cli.set_fan_duty(next, None).await {
                Ok(_) => {
                    last_duty = Some(next);
                    last_temp = Some(temp_c);
                }
                Err(e) => warn!("fan duty set failed: {}", e),
            }
        }

        sleep(Duration::from_millis(snapshot.poll_ms)).await;
    }

    // never reached in current simple model
}

fn map_temp_to_duty(temp_c: i32, points: &[[u32; 2]]) -> u32 {
    if points.is_empty() { return 0; }
    let t = temp_c.max(i32::MIN) as i64;
    let mut last = (points[0][0] as i64, points[0][1] as i64);
    for p in &points[1..] {
        let x = p[0] as i64; let y = p[1] as i64;
        if t <= x { return lerp(last, (x, y), t) as u32; }
        last = (x, y);
    }
    last.1 as u32
}

fn lerp(a: (i64, i64), b: (i64, i64), x: i64) -> i64 {
    let (x0, y0) = a; let (x1, y1) = b;
    if x1 == x0 { return y1; }
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

fn apply_rate_limit(prev: u32, target: u32, max_step: u32) -> u32 {
    if max_step == 0 { return target; }
    if target > prev { prev.saturating_add(max_step).min(target) }
    else { prev.saturating_sub(max_step).max(target) }
}

async fn read_temperature(cli: &FrameworkTool, prefer: &str) -> Option<i32> {
    let out = cli.power().await.ok()?; // minimal dependency; alternatively use --thermal later
    // Prefer a simple regex-free parse: look for lines like "APU:   62 C" or "CPU:   55 C"
    // Fallback: search for a number followed by ' C'
    parse_temp(&out, prefer).or_else(|| parse_temp(&out, "APU")).or_else(|| parse_any_temp(&out))
}

fn parse_temp(text: &str, label: &str) -> Option<i32> {
    let needle = format!("{}:", label);
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with(&needle) {
            // take last number before ' C'
            if let Some(idx) = l.rfind('C') {
                let sub = &l[..idx];
                let num = sub.chars().rev().take_while(|c| c.is_ascii_digit() || *c==' ').collect::<String>();
                let num = num.chars().rev().collect::<String>().trim().to_string();
                if let Ok(v) = num.parse::<i32>() { return Some(v); }
            }
        }
    }
    None
}

fn parse_any_temp(text: &str) -> Option<i32> {
    for line in text.lines() {
        let l = line.trim();
        if l.ends_with('C') {
            let sub = l.trim_end_matches('C').trim();
            if let Some(num) = sub.split_whitespace().last() {
                if let Ok(v) = num.parse::<i32>() { return Some(v); }
            }
        }
    }
    None
}


