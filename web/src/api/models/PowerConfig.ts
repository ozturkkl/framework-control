/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { PowerProfile } from './PowerProfile';
export type PowerConfig = {
    /**
     * Profile used when AC power is present (plugged in / charging)
     */
    ac?: PowerProfile;
    /**
     * Profile used when running on battery (not charging)
     */
    battery?: PowerProfile;
};

