use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ThermalParsed {
    pub temps: std::collections::BTreeMap<String, i32>,
    pub rpms: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct PowerBatteryInfo {
    pub last_full_charge_capacity_mah: Option<u32>,
    pub remaining_capacity_mah: Option<u32>,
    pub percentage: Option<u32>,
    pub present_voltage_mv: Option<u32>,
    pub present_rate_ma: Option<u32>,
    pub cycle_count: Option<u32>,
    pub charging: Option<bool>,
    pub discharging: Option<bool>,
}
pub fn parse_thermal(stdout: &str) -> ThermalParsed {
    let mut temps: std::collections::BTreeMap<String, i32> = Default::default();
    let mut rpms: Vec<u32> = vec![];
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
        if let Some(pos) = l.find("Fan Speed:") {
            let rest = &l[pos + "Fan Speed:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
                if let Ok(v) = tok.parse::<u32>() {
                    if v > 0 {
                        rpms.push(v);
                    }
                }
            }
        }
    }
    ThermalParsed { temps, rpms }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct PowerParsed {
    pub ac_present: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery: Option<PowerBatteryInfo>,
}
pub fn parse_power(stdout: &str) -> PowerParsed {
    let mut ac_present: Option<bool> = None;
    let mut last_full_charge_capacity_mah: Option<u32> = None;
    let mut remaining_capacity_mah: Option<u32> = None;
    let mut present_voltage_mv: Option<u32> = None;
    let mut present_rate_ma: Option<u32> = None;
    let mut cycle_count: Option<u32> = None;
    let mut charging: Option<bool> = None;
    let mut discharging: Option<bool> = None;
    let mut percentage: Option<u32> = None;

    for line in stdout.lines() {
        let l = line.trim();
        if l.starts_with("AC is:") {
            ac_present = Some(
                l.to_ascii_lowercase().contains("connected")
                    && !l.to_ascii_lowercase().contains("not connected"),
            );
        }
        if let Some(pos) = l.find("Battery LFCC:") {
            let rest = &l[pos + "Battery LFCC:".len()..];
            if let Some(num) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
                last_full_charge_capacity_mah = num.parse::<u32>().ok();
            }
        }
        if let Some(pos) = l.find("Battery Capacity:") {
            let rest = &l[pos + "Battery Capacity:".len()..];
            if let Some(num) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
                remaining_capacity_mah = num.parse::<u32>().ok();
            }
        }
        if let Some(pos) = l.find("Charge level:") {
            let rest = &l[pos + "Charge level:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.trim_end_matches('%').chars().all(|c| c.is_ascii_digit()))
            {
                percentage = tok.trim_end_matches('%').parse::<u32>().ok();
            }
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
        }
        if let Some(pos) = l.find("Charger Voltage:") {
            let rest = &l[pos + "Charger Voltage:".len()..];
            if let Some(tok) = rest.split_whitespace().find(|t| {
                t.ends_with("mV") && t.trim_end_matches("mV").chars().all(|c| c.is_ascii_digit())
            }) {
                present_voltage_mv = tok.trim_end_matches("mV").parse::<u32>().ok();
            }
        }
        if let Some(pos) = l.find("Present Rate:") {
            let rest = &l[pos + "Present Rate:".len()..];
            if let Some(tok) = rest
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
                present_rate_ma = tok.parse::<u32>().ok();
            }
        }
        if let Some(pos) = l.find("Charger Current:") {
            let rest = &l[pos + "Charger Current:".len()..];
            if let Some(tok) = rest.split_whitespace().find(|t| {
                t.ends_with("mA") && t.trim_end_matches("mA").chars().all(|c| c.is_ascii_digit())
            }) {
                present_rate_ma = tok.trim_end_matches("mA").parse::<u32>().ok();
            }
        }
        if l.starts_with("Cycle Count:") {
            if let Some(tok) = l
                .split_whitespace()
                .find(|t| t.chars().all(|c| c.is_ascii_digit()))
            {
                cycle_count = tok.parse::<u32>().ok();
            }
        }
        if l.eq_ignore_ascii_case("Battery charging") {
            charging = Some(true);
        }
        if l.eq_ignore_ascii_case("Battery discharging") {
            discharging = Some(true);
        }
    }

    let battery = if last_full_charge_capacity_mah.is_some()
        || remaining_capacity_mah.is_some()
        || percentage.is_some()
        || present_voltage_mv.is_some()
        || present_rate_ma.is_some()
        || cycle_count.is_some()
        || charging.is_some()
        || discharging.is_some()
    {
        Some(PowerBatteryInfo {
            last_full_charge_capacity_mah,
            remaining_capacity_mah,
            percentage,
            present_voltage_mv,
            present_rate_ma,
            cycle_count,
            charging,
            discharging,
        })
    } else {
        None
    };

    PowerParsed {
        ac_present,
        battery,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct VersionsParsed {
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

    #[test]
    fn parse_thermal_basic() {
        let s = "  F75303_Local:   45 C\n  F75303_CPU:     55 C\n  APU:          62 C\n  Fan Speed:  3171 RPM\n";
        let t = parse_thermal(s);
        assert_eq!(t.temps.get("APU").copied(), Some(62));
        assert_eq!(t.temps.get("F75303_CPU").copied(), Some(55));
        assert_eq!(t.rpms, vec![3171]);
    }
}
