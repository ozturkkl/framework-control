use super::*;

#[test]
fn parse_info_takes_lowest_power_limit_and_core_thermal_limit() {
    let s = r#"CPU Family: Phoenix
SMU BIOS Interface Version: 24
Version: v0.16.0 
PM Table Version: 4c0008
|        Name         |   Value   |     Parameter      |
|---------------------|-----------|--------------------|
| STAPM LIMIT         |    28.000 | stapm-limit        |
| STAPM VALUE         |     6.588 |                    |
| PPT LIMIT FAST      |    35.000 | fast-limit         |
| PPT VALUE FAST      |     9.217 |                    |
| PPT LIMIT SLOW      |    30.000 | slow-limit         |
| PPT VALUE SLOW      |     6.997 |                    |
| StapmTimeConst      |   275.000 | stapm-time         |
| SlowPPTTimeConst    |     5.000 | slow-time          |
| PPT LIMIT APU       |    54.000 | apu-slow-limit     |
| PPT VALUE APU       |     0.000 |                    |
| THM LIMIT CORE      |   100.000 | tctl-temp          |
| THM VALUE CORE      |    48.410 |                    |
| STT LIMIT APU       |    45.000 | apu-skin-temp      |
| STT VALUE APU       |    37.413 |                    |
| CCLK Boost SETPOINT |    95.000 | power-saving /     |
| CCLK BUSY VALUE     |    16.598 | max-performance    |
PM Table Dump of Version: 4c0008
| Offset |    Data    |   Value   |
|--------|------------|-----------|
| 0x0000 | 0x41E00000 |    28.000 |
| 0x0004 | 0x40D2D0E5 |     6.588 |
"#;
    let info = parse_info(s);
    assert_eq!(info.tdp_watts, Some(28));
    assert_eq!(info.thermal_limit_c, Some(100));
}
