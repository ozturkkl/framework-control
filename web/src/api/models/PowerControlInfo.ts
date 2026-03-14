/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { PowerCapabilities } from './PowerCapabilities';
import type { PowerState } from './PowerState';
export type PowerControlInfo = {
    /**
     * What capabilities are available
     */
    capabilities: PowerCapabilities;
    /**
     * Current state (what's actually applied right now)
     */
    current_state: PowerState;
};

