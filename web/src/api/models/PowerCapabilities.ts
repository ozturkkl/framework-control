/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
export type PowerCapabilities = {
    supports_tdp: boolean;
    supports_thermal: boolean;
    supports_epp: boolean;
    supports_governor: boolean;
    supports_frequency_limits: boolean;
    available_epp_preferences?: Array<string>;
    available_governors?: Array<string>;
    frequency_min_mhz?: number;
    frequency_max_mhz?: number;
    tdp_min_watts?: number;
    tdp_max_watts?: number;
};

