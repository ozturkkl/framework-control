use super::*;

#[test]
fn curve_interpolates_between_points_and_implicit_anchors() {
    let points = [[40, 20], [60, 40], [75, 80]];
    assert_eq!(calculate_duty_from_curve(0, &points), 0);
    assert_eq!(calculate_duty_from_curve(20, &points), 10);
    assert_eq!(calculate_duty_from_curve(40, &points), 20);
    assert_eq!(calculate_duty_from_curve(50, &points), 30);
    assert_eq!(calculate_duty_from_curve(75, &points), 80);
    assert_eq!(calculate_duty_from_curve(87, &points), 90);
    assert_eq!(calculate_duty_from_curve(100, &points), 100);
    assert_eq!(calculate_duty_from_curve(120, &points), 100);

    assert_eq!(calculate_duty_from_curve(50, &[]), 50);
}

#[test]
fn unsorted_points_evaluate_like_sorted() {
    assert_eq!(calculate_duty_from_curve(50, &[[60, 40], [40, 20]]), 30);
}

#[test]
fn rate_limit_caps_step_without_overshooting() {
    assert_eq!(apply_rate_limit(30, 50, 10), 40);
    assert_eq!(apply_rate_limit(30, 35, 10), 35);
    assert_eq!(apply_rate_limit(50, 30, 10), 40);
    assert_eq!(apply_rate_limit(50, 45, 10), 45);
}

#[test]
fn max_sensor_temperature_falls_back_only_when_no_sensor_matches() {
    let temps = [("apu_f75303@4d".to_string(), 58), ("cpu@4c".to_string(), 62)].into();
    // Names from framework_tool <= 0.6.5 no longer exist.
    assert_eq!(max_sensor_temperature(&temps, &["APU".to_string()]), Some(62));
    // A temporarily missing sensor (unpowered dGPU) shouldn't trigger the fallback.
    assert_eq!(
        max_sensor_temperature(&temps, &["apu_f75303@4d".to_string(), "gpu_temp@40".to_string()]),
        Some(58)
    );
}

fn curve(points: Vec<[u32; 2]>, hysteresis_c: u32, rate_limit_pct_per_step: u32) -> CurveConfig {
    CurveConfig {
        sensors: vec![],
        points,
        hysteresis_c,
        rate_limit_pct_per_step,
        rate_limit_down_pct_per_step: None,
    }
}

#[test]
fn stepper_repeats_duty_until_applied() {
    let c = curve(vec![[40, 20], [60, 40]], 0, 100);
    let mut s = CurveStepper::new();
    assert_eq!(s.next(50, &c), Some(30));
    assert_eq!(s.next(50, &c), Some(30));
    s.note_applied(30);
    assert_eq!(s.next(50, &c), None);
}

#[test]
fn stepper_hysteresis_blocks_small_drops() {
    let c = curve(vec![[40, 20], [60, 40]], 5, 100);
    let mut s = CurveStepper::new();
    s.note_applied(40);
    assert_eq!(s.next(60, &c), None);
    assert_eq!(s.next(58, &c), None);
    assert_eq!(s.next(54, &c), Some(34));
}

#[test]
fn stepper_hysteresis_never_delays_spin_up() {
    let c = curve(vec![[40, 20], [60, 40]], 5, 100);
    let mut s = CurveStepper::new();
    s.note_applied(40);
    assert_eq!(s.next(60, &c), None);
    assert_eq!(s.next(58, &c), None);
    assert_eq!(s.next(61, &c), Some(42));
}

#[test]
fn stepper_reset_drops_hysteresis_anchor() {
    let c = curve(vec![[40, 20], [60, 40]], 5, 100);
    let mut s = CurveStepper::new();
    s.note_applied(40);
    assert_eq!(s.next(60, &c), None);
    s.reset();
    assert_eq!(s.next(58, &c), Some(38));
}

#[test]
fn stepper_rate_limit_steps_toward_target() {
    let c = curve(vec![[40, 20], [60, 80]], 0, 10);
    let mut s = CurveStepper::new();
    s.note_applied(20);
    assert_eq!(s.next(60, &c), Some(30));
}

#[test]
fn stepper_separate_down_rate_limit() {
    let mut c = curve(vec![[40, 20], [60, 80]], 0, 100);
    c.rate_limit_down_pct_per_step = Some(5);
    let mut s = CurveStepper::new();
    s.note_applied(80);
    assert_eq!(s.next(40, &c), Some(75));
}
