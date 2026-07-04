use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ThermalParsed {
    pub temps: std::collections::BTreeMap<String, i32>,
    pub fans: Vec<FanReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct FanReading {
    pub name: String,
    pub rpm: u32,
}

// EC fan-speed sentinels: 0xFFFE = stalled (printed as a bogus 65534), 0xFFFF =
const EC_FAN_SENTINEL_MIN: u32 = 0xFFFE;
#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct PowerBatteryInfo {
    pub ac_present: Option<bool>,
    pub battery_present: Option<bool>,
    pub last_full_charge_capacity_mah: Option<u32>,
    pub remaining_capacity_mah: Option<u32>,
    pub percentage: Option<u32>,
    pub soc_pct: Option<u32>,
    pub present_voltage_mv: Option<u32>,
    pub present_rate_ma: Option<u32>,
    pub charger_voltage_mv: Option<u32>,
    pub charger_current_ma: Option<u32>,
    pub charge_input_current_ma: Option<u32>,
    pub design_capacity_mah: Option<u32>,
    pub design_voltage_mv: Option<u32>,
    pub cycle_count: Option<u32>,
    pub charging: Option<bool>,
    pub discharging: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct BatteryChargeLimitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_limit_min_pct: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charge_limit_max_pct: Option<u8>,
}
pub fn parse_thermal(stdout: &str) -> ThermalParsed {
    let mut temps: std::collections::BTreeMap<String, i32> = Default::default();
    let mut fans: Vec<FanReading> = vec![];
    for line in stdout.lines() {
        let l = line.trim();
        if let Some((k, r)) = l.split_once(':') {
            let key = k.trim();
            if let Some(c_pos) = r.rfind('C') {
                let before_c = &r[..c_pos];
                if let Some(tok) = before_c.split_whitespace().last() {
                    if let Ok(val) = tok.parse::<i32>() {
                        temps.insert(key.to_string(), val);
                        continue;
                    }
                }
            }
        }
        if let Some(reading) = parse_fan_line(l, fans.len()) {
            fans.push(reading);
        }
    }
    ThermalParsed { temps, fans }
}

fn parse_fan_line(line: &str, position: usize) -> Option<FanReading> {
    let rpm_pos = line.find("RPM")?;
    let before = &line[..rpm_pos];
    let raw: u32 = before.split_whitespace().last()?.parse().ok()?;

    let rpm = if raw >= EC_FAN_SENTINEL_MIN || line.contains("Stalled") {
        0
    } else {
        raw
    };

    let name = line
        .split_once(':')
        .map(|(n, _)| n.trim())
        .filter(|n| !n.is_empty())
        .map(|n| n.to_string())
        .unwrap_or_else(|| format!("Fan {}", position + 1));

    Some(FanReading { name, rpm })
}

/// Parse output of `framework_tool --charge-limit` which prints: "Minimum X%, Maximum Y%"
pub fn parse_charge_limit(stdout: &str) -> BatteryChargeLimitInfo {
    let mut info = BatteryChargeLimitInfo::default();
    for line in stdout.lines() {
        let l = line.trim().to_ascii_lowercase();
        if l.contains("minimum") {
            if let Some(p) = l.split_whitespace().find(|t| t.ends_with('%')) {
                let n = p.trim_end_matches('%').parse::<u8>().ok();
                if let Some(v) = n {
                    info.charge_limit_min_pct = Some(v);
                }
            } else {
                // fallback: scan digits
                if let Some(tok) = l
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .find(|t| t.chars().all(|c| c.is_ascii_digit()))
                {
                    info.charge_limit_min_pct = tok.parse::<u8>().ok();
                }
            }
        }
        if l.contains("maximum") {
            if let Some(p) = l.split_whitespace().find(|t| t.ends_with('%')) {
                let n = p.trim_end_matches('%').parse::<u8>().ok();
                if let Some(v) = n {
                    info.charge_limit_max_pct = Some(v);
                }
            } else {
                if let Some(tok) = l
                    .split(|c: char| !c.is_ascii_alphanumeric())
                    .find(|t| t.chars().all(|c| c.is_ascii_digit()))
                {
                    info.charge_limit_max_pct = tok.parse::<u8>().ok();
                }
            }
        }
    }
    info
}
pub fn parse_power(stdout: &str) -> PowerBatteryInfo {
    let mut ac_present: Option<bool> = None;
    let mut battery_present: Option<bool> = None;
    let mut last_full_charge_capacity_mah: Option<u32> = None;
    let mut remaining_capacity_mah: Option<u32> = None;
    let mut present_voltage_mv: Option<u32> = None;
    let mut present_rate_ma: Option<u32> = None;
    let mut charger_voltage_mv: Option<u32> = None;
    let mut charger_current_ma: Option<u32> = None;
    let mut charge_input_current_ma: Option<u32> = None;
    let mut design_capacity_mah: Option<u32> = None;
    let mut design_voltage_mv: Option<u32> = None;
    let mut cycle_count: Option<u32> = None;
    let mut charging: Option<bool> = None;
    let mut discharging: Option<bool> = None;
    let mut percentage: Option<u32> = None;
    let mut soc_pct: Option<u32> = None;

    for line in stdout.lines() {
        let l = line.trim();
        if l.starts_with("AC is:") {
            ac_present =
                Some(l.to_ascii_lowercase().contains("connected") && !l.to_ascii_lowercase().contains("not connected"));
            continue;
        }
        if l.starts_with("Battery is:") {
            battery_present =
                Some(l.to_ascii_lowercase().contains("connected") && !l.to_ascii_lowercase().contains("not connected"));
            continue;
        }
        if let Some(pos) = l.find("Battery LFCC:") {
            let rest = &l[pos + "Battery LFCC:".len()..];
            if let Some(num) = rest.split_whitespace().find(|t| t.chars().all(|c| c.is_ascii_digit())) {
                last_full_charge_capacity_mah = num.parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Battery Capacity:") {
            let rest = &l[pos + "Battery Capacity:".len()..];
            if let Some(num) = rest.split_whitespace().find(|t| t.chars().all(|c| c.is_ascii_digit())) {
                remaining_capacity_mah = num.parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Charge level:") {
            let rest = &l[pos + "Charge level:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.trim_end_matches('%').chars().all(|c| c.is_ascii_digit()))
            {
                percentage = tok.trim_end_matches('%').parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Battery SoC:") {
            let rest = &l[pos + "Battery SoC:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.trim_end_matches('%').chars().all(|c| c.is_ascii_digit()))
            {
                soc_pct = tok.trim_end_matches('%').parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Present Voltage:") {
            let rest = &l[pos + "Present Voltage:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit() || c == '.'))
            {
                if let Ok(v) = tok.parse::<f32>() {
                    present_voltage_mv = Some((v * 1000.0) as u32);
                }
            }
            continue;
        }
        if let Some(pos) = l.find("Charger Voltage:") {
            let rest = &l[pos + "Charger Voltage:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.ends_with("mV") && t.trim_end_matches("mV").chars().all(|c| c.is_ascii_digit()))
            {
                charger_voltage_mv = tok.trim_end_matches("mV").parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Present Rate:") {
            let rest = &l[pos + "Present Rate:".len()..];
            if let Some(tok) = rest.split_whitespace().find(|t| t.chars().all(|c| c.is_ascii_digit())) {
                present_rate_ma = tok.parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Charger Current:") {
            let rest = &l[pos + "Charger Current:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.ends_with("mA") && t.trim_end_matches("mA").chars().all(|c| c.is_ascii_digit()))
            {
                charger_current_ma = tok.trim_end_matches("mA").parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Chg Input Current:") {
            let rest = &l[pos + "Chg Input Current:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.ends_with("mA") && t.trim_end_matches("mA").chars().all(|c| c.is_ascii_digit()))
            {
                charge_input_current_ma = tok.trim_end_matches("mA").parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Design Capacity:") {
            let rest = &l[pos + "Design Capacity:".len()..];
            if let Some(num) = rest.split_whitespace().find(|t| t.chars().all(|c| c.is_ascii_digit())) {
                design_capacity_mah = num.parse::<u32>().ok();
            }
            continue;
        }
        if let Some(pos) = l.find("Design Voltage:") {
            let rest = &l[pos + "Design Voltage:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit() || c == '.'))
            {
                if let Ok(v) = tok.parse::<f32>() {
                    design_voltage_mv = Some((v * 1000.0) as u32);
                }
            }
            continue;
        }
        if l.starts_with("Cycle Count:") {
            if let Some(tok) = l.split_whitespace().find(|t| t.chars().all(|c| c.is_ascii_digit())) {
                cycle_count = tok.parse::<u32>().ok();
            }
            continue;
        }
        if l.eq_ignore_ascii_case("Battery charging") {
            charging = Some(true);
            continue;
        }
        if l.eq_ignore_ascii_case("Battery discharging") {
            discharging = Some(true);
            continue;
        }
    }

    PowerBatteryInfo {
        ac_present,
        battery_present,
        last_full_charge_capacity_mah,
        remaining_capacity_mah,
        percentage,
        soc_pct,
        present_voltage_mv,
        present_rate_ma,
        charger_voltage_mv,
        charger_current_ma,
        charge_input_current_ma,
        design_capacity_mah,
        design_voltage_mv,
        cycle_count,
        charging,
        discharging,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct VersionsParsed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mainboard_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mainboard_revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uefi_release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_build_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ec_current_image: Option<String>,
}
pub fn parse_versions(text: &str) -> VersionsParsed {
    let mut out = VersionsParsed::default();
    if text.is_empty() {
        return out;
    }
    let mut section: String = String::new();
    for raw in text.lines() {
        let line = raw.replace('\t', "    ");
        if line.trim().is_empty() {
            continue;
        }
        if let Some(v) = line.trim().strip_prefix("Tool Version:") {
            out.tool_version = Some(v.trim().to_string());
            continue;
        }
        let is_section = !line.starts_with(' ') && !line.starts_with('\t');
        if is_section {
            section = line.trim().to_string();
            continue;
        }
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            continue;
        }
        let key = parts[0].trim().to_ascii_lowercase();
        let value = parts[1].trim().to_string();
        let s = section.to_ascii_lowercase();
        if s.starts_with("mainboard hardware") {
            if key == "type" {
                out.mainboard_type = Some(value);
            } else if key == "revision" {
                out.mainboard_revision = Some(value);
            }
        } else if s.starts_with("uefi bios") {
            if key == "version" {
                out.uefi_version = Some(value);
            } else if key == "release date" {
                out.uefi_release_date = Some(value);
            }
        } else if s.starts_with("ec firmware") {
            if key == "build version" {
                out.ec_build_version = Some(value);
            } else if key == "current image" {
                out.ec_current_image = Some(value);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rpms(t: &ThermalParsed) -> Vec<u32> {
        t.fans.iter().map(|f| f.rpm).collect()
    }

    #[test]
    fn parse_thermal_basic() {
        let s = r#"
  F75303_Local:   45 C
  F75303_CPU:     55 C
  APU:            62 C
  Fan Speed:    3171 RPM
"#;
        let t = parse_thermal(s);
        assert_eq!(t.temps.get("APU").copied(), Some(62));
        assert_eq!(t.temps.get("F75303_CPU").copied(), Some(55));
        assert_eq!(rpms(&t), vec![3171]);
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
    fn parse_thermal_multi_fan_keeps_position() {
        let s = r#"
  APU:          50 C
  Fan Speed:     0 RPM
  Fan Speed:  2917 RPM
"#;
        let t = parse_thermal(s);
        assert_eq!(rpms(&t), vec![0, 2917]);
    }
    #[test]
    fn parse_thermal_named_fans() {
        let s = r#"
  APU:          50 C
  Left Fan:   1200 RPM
  Right Fan:  1300 RPM
"#;
        let t = parse_thermal(s);
        assert_eq!(rpms(&t), vec![1200, 1300]);
        let names: Vec<&str> = t.fans.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["Left Fan", "Right Fan"]);
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
    fn parse_thermal_ignores_non_fan_rpm_lines() {
        // A line mentioning RPM without a numeric reading before it is not a fan.
        let s = r#"
  Note: fans report in RPM
  Left Fan:  1200 RPM
"#;
        let t = parse_thermal(s);
        assert_eq!(rpms(&t), vec![1200]);
    }
    #[test]
    fn parse_versions_tool_version() {
        let s = r#"
Tool Version:     0.6.4
OS Version:       Linux 7.1.1 x86_64
Mainboard Hardware
  Type:           Laptop 13 (AMD Ryzen 7040Series)
  Revision:       MassProduction
"#;
        let v = parse_versions(s);
        assert_eq!(v.tool_version.as_deref(), Some("0.6.4"));
        assert_eq!(v.mainboard_type.as_deref(), Some("Laptop 13 (AMD Ryzen 7040Series)"));
    }
    #[test]
    fn parse_power_verbose_sample() {
        let s = r#"
Charger Status
  AC is:            connected
  Charger Voltage:  17800mV
  Charger Current:  3568mA
  Chg Input Current:4400mA
  Battery SoC:      52%
Battery Status
  AC is:            connected
  Battery is:       connected
  Battery LFCC:     5182 mAh (Last Full Charge Capacity)
  Battery Capacity: 2685 mAh
  Charge level:     51%
  Manufacturer:     NVT
  Model Number:     FRANDBA
  Serial Number:    0204
  Battery Type:     LION
  Present Voltage:  16.591 V
  Present Rate:     3221 mA
  Design Capacity:  5491 mAh
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
}
