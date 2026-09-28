use std::collections::VecDeque;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use tokio::time::{sleep, Duration};
use tracing::{info, warn};

use crate::cli::framework_tool_parser::PowerBatteryInfo;
use crate::cli::FrameworkTool;
use crate::config::LiveConfig;
use crate::types::{BatterySample, DashboardPanelId};
use crate::utils::time::unix_time_ms;

const RETAIN_SECONDS: u64 = 7 * 24 * 3600;
const SAVE_DEBOUNCE: Duration = Duration::from_secs(60);

pub async fn run(
    cli_lock: Arc<tokio::sync::RwLock<Option<FrameworkTool>>>,
    cfg_lock: LiveConfig,
    samples_lock: Arc<tokio::sync::RwLock<VecDeque<BatterySample>>>,
) {
    info!("Battery history task started");
    let path = crate::config::battery_history_path();
    {
        let mut w = samples_lock.write().await;
        *w = load_samples(&path);
        trim(&mut w, unix_time_ms());
    }
    let mut last_save: Option<Instant> = None;

    loop {
        let (poll_ms, battery_enabled) = {
            let cfg = cfg_lock.read().await;
            (
                cfg.battery.history_poll_ms(),
                cfg.ui.is_panel_enabled(DashboardPanelId::Battery),
            )
        };
        let poll_interval = Duration::from_millis(poll_ms);

        if !battery_enabled {
            sleep(poll_interval).await;
            continue;
        }

        let maybe_cli = { cli_lock.read().await.clone() };
        let Some(cli) = maybe_cli else {
            sleep(poll_interval).await;
            continue;
        };

        match cli.power().await {
            Ok(info) => {
                let now_ms = unix_time_ms();
                if let Some(sample) = sample_from(&info, now_ms) {
                    let snapshot = {
                        let mut w = samples_lock.write().await;
                        record(&mut w, sample);
                        let due = last_save.map(|t| t.elapsed() >= SAVE_DEBOUNCE).unwrap_or(true);
                        due.then(|| w.clone())
                    };
                    if let Some(snapshot) = snapshot {
                        match save_samples(&path, &snapshot) {
                            Ok(()) => last_save = Some(Instant::now()),
                            Err(e) => warn!("battery history save failed: {}", e),
                        }
                    }
                }
            }
            Err(e) => {
                warn!("battery history read failed: {}", e);
            }
        }

        sleep(poll_interval).await;
    }
}

fn record(samples: &mut VecDeque<BatterySample>, sample: BatterySample) {
    trim(samples, sample.ts_ms);
    if samples.back().map(|back| sample.ts_ms > back.ts_ms).unwrap_or(true) {
        samples.push_back(sample);
    }
}

fn trim(samples: &mut VecDeque<BatterySample>, now_ms: i64) {
    let cutoff_ms = now_ms - (RETAIN_SECONDS as i64 * 1000);
    while let Some(front) = samples.front() {
        if front.ts_ms < cutoff_ms {
            samples.pop_front();
        } else {
            break;
        }
    }
    // Drop timestamps ahead of now so an NTP/RTC step back can record again.
    while let Some(back) = samples.back() {
        if back.ts_ms > now_ms {
            samples.pop_back();
        } else {
            break;
        }
    }
}

fn load_samples(path: &Path) -> VecDeque<BatterySample> {
    let buf = match std::fs::read_to_string(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return VecDeque::new(),
        Err(e) => {
            warn!("battery history unreadable at {:?}: {}", path, e);
            return VecDeque::new();
        }
        Ok(s) => s,
    };
    match serde_json::from_str::<Vec<BatterySample>>(&buf) {
        Ok(v) => {
            info!("Loaded {} battery history samples from {:?}", v.len(), path);
            VecDeque::from(v)
        }
        Err(e) => {
            warn!("battery history corrupt at {:?}: {}", path, e);
            VecDeque::new()
        }
    }
}

fn save_samples(path: &Path, samples: &VecDeque<BatterySample>) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(samples).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

fn sample_from(info: &PowerBatteryInfo, now_ms: i64) -> Option<BatterySample> {
    let charge_pct = charge_pct(info);
    let watts = signed_watts_and_charging(info);
    if charge_pct.is_none() && watts.is_none() {
        return None;
    }
    Some(BatterySample {
        ts_ms: now_ms,
        charge_pct,
        watts,
        ac_present: info.ac_present,
    })
}

fn charge_pct(info: &PowerBatteryInfo) -> Option<f32> {
    match (info.remaining_capacity_mah, info.last_full_charge_capacity_mah) {
        (Some(rem), Some(full)) if full > 0 => Some(((rem as f32 / full as f32) * 100.0).clamp(0.0, 100.0)),
        _ => info.percentage.or(info.soc_pct).map(|v| v as f32),
    }
}

fn signed_watts_and_charging(info: &PowerBatteryInfo) -> Option<f32> {
    let (ma, mv) = (info.present_rate_ma?, info.present_voltage_mv?);
    let mag = (ma as f32) * (mv as f32) / 1_000_000.0;
    if mag.abs() < f32::EPSILON {
        return Some(0.0);
    }
    let sign = match (info.charging, info.discharging) {
        (Some(true), _) => 1.0,
        (Some(false), _) | (_, Some(true)) => -1.0,
        _ => return None,
    };
    Some(mag.abs() * sign)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info() -> PowerBatteryInfo {
        PowerBatteryInfo {
            remaining_capacity_mah: Some(40000),
            last_full_charge_capacity_mah: Some(50000),
            present_rate_ma: Some(2000),
            present_voltage_mv: Some(15000),
            charging: Some(true),
            ac_present: Some(true),
            ..Default::default()
        }
    }

    #[test]
    fn discharging_flag_signs_negative() {
        let mut i = info();
        i.charging = None;
        i.discharging = Some(true);
        let s = sample_from(&i, 1_000).unwrap();
        assert!((s.watts.unwrap() + 30.0).abs() < 0.01);
    }

    #[test]
    fn missing_direction_does_not_invent_discharge() {
        let mut i = info();
        i.charging = None;
        i.discharging = None;
        let s = sample_from(&i, 1_000).unwrap();
        assert_eq!(s.watts, None);
        assert!(s.charge_pct.is_some());
    }

    #[test]
    fn zero_rate_is_zero_watts() {
        let mut i = info();
        i.present_rate_ma = Some(0);
        i.charging = None;
        i.discharging = None;
        let s = sample_from(&i, 1_000).unwrap();
        assert_eq!(s.watts, Some(0.0));
    }
}
