/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { BatteryInfo } from './BatteryInfo';
import type { PowerControlInfo } from './PowerControlInfo';
export type PowerResponse = {
    /**
     * Battery info (framework_tool --power) + charge limits (charge-limit CLI)
     */
    battery?: BatteryInfo;
    /**
     * Power control information
     */
    power_control: PowerControlInfo;
};

