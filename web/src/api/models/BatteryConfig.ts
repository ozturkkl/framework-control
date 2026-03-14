/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { SettingF32 } from './SettingF32';
import type { SettingU8 } from './SettingU8';
export type BatteryConfig = {
    /**
     * EC charge limit maximum percent (25-100)
     */
    charge_limit_max_pct?: SettingU8;
    /**
     * Charge rate in C (0.05 - 1.0). When disabled, use 1.0C to approximate no limit.
     */
    charge_rate_c?: SettingF32;
    /**
     * Optional SoC threshold (%) for rate limiting
     */
    charge_rate_soc_threshold_pct?: number;
};

