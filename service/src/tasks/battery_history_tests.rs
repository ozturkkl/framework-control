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

fn sample(ts_ms: i64) -> BatterySample {
    BatterySample {
        ts_ms,
        charge_pct: Some(80.0),
        watts: Some(-12.5),
        ac_present: Some(false),
    }
}

fn timestamps(samples: &VecDeque<BatterySample>) -> Vec<i64> {
    samples.iter().map(|s| s.ts_ms).collect()
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

#[test]
fn record_drops_samples_older_than_retention_window() {
    let window_ms = RETAIN_SECONDS as i64 * 1000;
    let now = 10 * window_ms;
    let cutoff = now - window_ms;
    let mut samples = VecDeque::from([sample(cutoff - 1), sample(cutoff), sample(now - 1)]);
    record(&mut samples, sample(now));
    assert_eq!(timestamps(&samples), [cutoff, now - 1, now]);
}

#[test]
fn clock_step_back_drops_future_samples() {
    let mut samples = VecDeque::from([sample(1_000), sample(2_000), sample(3_000)]);
    record(&mut samples, sample(1_500));
    assert_eq!(timestamps(&samples), [1_000, 1_500]);
}

#[test]
fn saved_samples_load_back() {
    let dir = std::env::temp_dir().join(format!("fc-battery-history-{}", std::process::id()));
    let path = dir.join("nested").join("battery_history.json");
    let samples = VecDeque::from([sample(1_000), sample(2_000)]);

    save_samples(&path, &samples).unwrap();
    let loaded = load_samples(&path);
    std::fs::remove_dir_all(&dir).unwrap();

    assert_eq!(timestamps(&loaded), [1_000, 2_000]);
}
