use poem_openapi::Object;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Object, Default)]
pub struct RyzenAdjInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tdp_watts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thermal_limit_c: Option<u32>,
}

/// Parse output of `ryzenadj --info --dump-table`
/// Strategy: scan table rows and extract key limits as watts (rounded)
pub fn parse_info(text: &str) -> RyzenAdjInfo {
    let mut info = RyzenAdjInfo::default();
    if text.is_empty() {
        return info;
    }
    let mut limits_w: Vec<f32> = Vec::new();

    // Matches lines like: | STAPM LIMIT         |    67.000 | stapm-limit        |
    // Columns: name | value | parameter
    let row_re =
        Regex::new(r"^\|\s*([^|]+?)\s*\|\s*([+-]?(?:\d+\.)?\d+)\s*\|\s*(?:[^|]*)\|\s*$").ok();
    if let Some(re) = row_re.as_ref() {
        for line in text.lines() {
            let l = line.trim();
            if !l.starts_with('|') || l.starts_with("|-") {
                continue;
            }
            if let Some(c) = re.captures(l) {
                let name = c
                    .get(1)
                    .map(|m| m.as_str().trim())
                    .unwrap_or("")
                    .to_ascii_uppercase();
                let val = c.get(2).and_then(|m| m.as_str().trim().parse::<f32>().ok());
                if let Some(v) = val {
                    // Collect power limit candidates
                    if name.contains("STAPM LIMIT")
                        || name.contains("PPT LIMIT FAST")
                        || name.contains("PPT LIMIT SLOW")
                    {
                        limits_w.push(v);
                    }
                    // Thermal limit
                    if name.contains("THM LIMIT CORE") || name.contains("TCTL") {
                        info.thermal_limit_c = Some(v.round() as u32);
                    }
                }
            }
        }
    }
    if !limits_w.is_empty() {
        let min_w = limits_w.into_iter().fold(f32::INFINITY, f32::min);
        if min_w.is_finite() {
            info.tdp_watts = Some(min_w.round().max(1.0) as u32);
        }
    }
    info
}
