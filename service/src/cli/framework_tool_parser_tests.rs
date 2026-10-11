use super::*;

fn rpms(t: &ThermalParsed) -> Vec<u32> {
    t.fans.iter().map(|f| f.rpm).collect()
}

#[test]
fn parse_thermal_framework16_sample() {
    // Real Framework 16 output: both fans labelled "Fan Speed", and a
    // "NotPowered" dGPU sensor that has no numeric reading.
    let s = r#"
  F75303_Local: 40 C
  F75303_CPU:   38 C
  F75303_DDR:   38 C
  APU:          39 C
  dGPU VR:      0 C
  dGPU VRAM:    0 C
  dGPU AMB:     0 C
  dGPU temp:    NotPowered
  Fan Speed:  2165 RPM
  Fan Speed:  2035 RPM
"#;
    let t = parse_thermal(s);
    assert_eq!(t.temps.get("APU").copied(), Some(39));
    assert_eq!(t.temps.get("dGPU VR").copied(), Some(0));
    assert_eq!(t.temps.get("dGPU temp"), None);
    assert_eq!(rpms(&t), vec![2165, 2035]);
    let names: Vec<&str> = t.fans.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["Fan Speed", "Fan Speed"]);
}
#[test]
fn parse_thermal_stalled_fan_is_zero_not_sentinel() {
    // Stalled fan (0xFFFE/65534) reads as 0 but keeps its slot.
    let s = r#"
  Left Fan:  65534 RPM (Stalled)
  Right Fan:  1300 RPM
"#;
    let t = parse_thermal(s);
    assert_eq!(rpms(&t), vec![0, 1300]);
    assert_eq!(t.fans[0].name, "Left Fan");
}
#[test]
fn parse_versions_full_sample() {
    let s = r#"
Tool Version:     0.4.5
OS Version:       Linux 6.12.10 #1-NixOS SMP PREEMPT_DYNAMIC x86_64
Mainboard Hardware
  Type:           Laptop 13 (AMD Ryzen 7040Series)
  Revision:       MassProduction
UEFI BIOS
  Version:        03.05
  Release Date:   03/29/2024
EC Firmware
  Build version:  lilac-3.0.3-413f018 2024-03-20 06:46:01 lotus@ip-172-26-3-226
  Current image:  RO
PD Controllers
  Right (01)
    Main:         0.0.0E (Active)
    Backup:       0.0.0E
  Left (23)
    Main:         0.0.0E (Active)
    Backup:       0.0.0E
"#;
    let v = parse_versions(s);
    assert_eq!(v.tool_version.as_deref(), Some("0.4.5"));
    assert_eq!(v.mainboard_type.as_deref(), Some("Laptop 13 (AMD Ryzen 7040Series)"));
    assert_eq!(v.mainboard_revision.as_deref(), Some("MassProduction"));
    assert_eq!(v.uefi_version.as_deref(), Some("03.05"));
    assert_eq!(v.uefi_release_date.as_deref(), Some("03/29/2024"));
    // The build string contains colons of its own (the timestamp).
    assert_eq!(
        v.ec_build_version.as_deref(),
        Some("lilac-3.0.3-413f018 2024-03-20 06:46:01 lotus@ip-172-26-3-226")
    );
    assert_eq!(v.ec_current_image.as_deref(), Some("RO"));
}
#[test]
fn parse_power_verbose_sample() {
    let s = r#"
Charger Status
  AC is:            connected
  Charger Voltage:  17800mV
  Charger Current:  3568mA
                    0.65C
  Chg Input Current:4400mA
  Battery SoC:      52%
Battery Status
  AC is:            connected
  Battery is:       connected
  Battery LFCC:     5182 mAh (Last Full Charge Capacity)
  Battery Capacity: 2685 mAh
                    44.546 Wh
  Charge level:     51%
  Manufacturer:     NVT
  Model Number:     FRANDBA
  Serial Number:    0204
  Battery Type:     LION
  Present Voltage:  16.591 V
  Present Rate:     3221 mA
  Design Capacity:  5491 mAh
                    85.0 Wh
  Design Voltage:   15.480 V
  Cycle Count:      58
  Battery charging
    "#;
    let p = parse_power(s);
    assert_eq!(p.ac_present, Some(true));
    assert_eq!(p.battery_present, Some(true));
    assert_eq!(p.last_full_charge_capacity_mah, Some(5182));
    assert_eq!(p.remaining_capacity_mah, Some(2685));
    assert_eq!(p.percentage, Some(51));
    assert_eq!(p.soc_pct, Some(52));
    assert_eq!(p.present_voltage_mv, Some(16591));
    assert_eq!(p.present_rate_ma, Some(3221));
    assert_eq!(p.charger_voltage_mv, Some(17800));
    assert_eq!(p.charger_current_ma, Some(3568));
    assert_eq!(p.charge_input_current_ma, Some(4400));
    assert_eq!(p.design_capacity_mah, Some(5491));
    assert_eq!(p.design_voltage_mv, Some(15480));
    assert_eq!(p.cycle_count, Some(58));
    assert_eq!(p.charging, Some(true));
}

#[test]
fn parse_power_desktop_without_battery() {
    let s = r#"
Battery Status
  AC is:            connected
  Battery is:       not connected
"#;
    let p = parse_power(s);
    assert_eq!(p.ac_present, Some(true));
    assert_eq!(p.battery_present, Some(false));
}

#[test]
fn parse_power_discharging_on_battery() {
    let s = r#"
Battery Status
  AC is:            not connected
  Battery is:       connected
  Battery LFCC:     5182 mAh (Last Full Charge Capacity)
  Battery Capacity: 4012 mAh
                    62.104 Wh
  Charge level:     77%
  Battery discharging
"#;
    let p = parse_power(s);
    assert_eq!(p.ac_present, Some(false));
    assert_eq!(p.percentage, Some(77));
    assert_eq!(p.discharging, Some(true));
}

#[test]
fn parse_power_voltage_remainder_is_not_zero_padded() {
    // framework_tool prints `{mv / 1000}.{mv % 1000}`, so 16051 mV appears as "16.51".
    let s = r#"
  Present Voltage:  16.51 V
  Design Voltage:   15.480 V
"#;
    let p = parse_power(s);
    assert_eq!(p.present_voltage_mv, Some(16051));
    assert_eq!(p.design_voltage_mv, Some(15480));
}

#[test]
fn parse_charge_limit_reads_min_and_max_from_one_line() {
    let c = parse_charge_limit("Minimum 0%, Maximum 80%\n");
    assert_eq!(c.charge_limit_min_pct, Some(0));
    assert_eq!(c.charge_limit_max_pct, Some(80));
}
