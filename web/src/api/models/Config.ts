/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { BatteryConfig } from './BatteryConfig';
import type { FanControlConfig } from './FanControlConfig';
import type { PowerConfig } from './PowerConfig';
import type { TelemetryConfig } from './TelemetryConfig';
import type { UiConfig } from './UiConfig';
import type { UpdatesConfig } from './UpdatesConfig';
export type Config = {
    fan: FanControlConfig;
    power: PowerConfig;
    battery: BatteryConfig;
    updates: UpdatesConfig;
    telemetry: TelemetryConfig;
    ui: UiConfig;
};

