/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { SettingString } from './SettingString';
import type { SettingU32 } from './SettingU32';
export type PowerProfile = {
    tdp_watts?: SettingU32;
    thermal_limit_c?: SettingU32;
    epp_preference?: SettingString;
    governor?: SettingString;
    min_freq_mhz?: SettingU32;
    max_freq_mhz?: SettingU32;
};

