/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { CurveConfig } from './CurveConfig';
import type { FanCalibration } from './FanCalibration';
import type { FanControlMode } from './FanControlMode';
import type { ManualConfig } from './ManualConfig';
export type FanControlConfig = {
    mode?: FanControlMode;
    manual?: ManualConfig;
    curve?: CurveConfig;
    calibration?: FanCalibration;
};

