/* generated using openapi-typescript-codegen -- do not edit */
/* istanbul ignore file */
/* tslint:disable */
/* eslint-disable */
import type { Config } from '../models/Config';
import type { Empty } from '../models/Empty';
import type { Health } from '../models/Health';
import type { PartialConfig } from '../models/PartialConfig';
import type { PowerResponse } from '../models/PowerResponse';
import type { ShortcutsStatus } from '../models/ShortcutsStatus';
import type { SystemInfo } from '../models/SystemInfo';
import type { TelemetrySample } from '../models/TelemetrySample';
import type { ThermalParsed } from '../models/ThermalParsed';
import type { UpdateCheck } from '../models/UpdateCheck';
import type { VersionsParsed } from '../models/VersionsParsed';
import type { CancelablePromise } from '../core/CancelablePromise';
import { OpenAPI } from '../core/OpenAPI';
import { request as __request } from '../core/request';
export class DefaultService {
    /**
     * Health: returns overall service health and CLI presence
     * @returns Health
     * @throws ApiError
     */
    public static health(): CancelablePromise<Health> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/health',
        });
    }
    /**
     * RyzenAdj: install on demand (Windows only)
     * @returns Empty
     * @throws ApiError
     */
    public static installRyzenadj(): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/ryzenadj/install',
        });
    }
    /**
     * RyzenAdj: uninstall and remove any downloaded artifacts (Windows only)
     * @returns Empty
     * @throws ApiError
     */
    public static uninstallRyzenadj(): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/ryzenadj/uninstall',
        });
    }
    /**
     * @returns PowerResponse
     * @throws ApiError
     */
    public static getPower(): CancelablePromise<PowerResponse> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/power',
        });
    }
    /**
     * Update: check for latest version from update feed
     * @returns UpdateCheck
     * @throws ApiError
     */
    public static checkUpdate(): CancelablePromise<UpdateCheck> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/update/check',
        });
    }
    /**
     * Update: apply latest by downloading MSI and invoking msiexec (Windows only)
     * @returns Empty
     * @throws ApiError
     */
    public static applyUpdate(): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/update/apply',
        });
    }
    /**
     * Thermal (parsed)
     * @returns ThermalParsed
     * @throws ApiError
     */
    public static getThermal(): CancelablePromise<ThermalParsed> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/thermal',
        });
    }
    /**
     * Telemetry history: returns recent samples collected by the service
     * @returns TelemetrySample
     * @throws ApiError
     */
    public static getThermalHistory(): CancelablePromise<Array<TelemetrySample>> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/thermal/history',
        });
    }
    /**
     * Framework versions (parsed)
     * @returns VersionsParsed
     * @throws ApiError
     */
    public static getVersions(): CancelablePromise<VersionsParsed> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/versions',
        });
    }
    /**
     * Get config
     * @returns Config
     * @throws ApiError
     */
    public static getConfig(): CancelablePromise<Config> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/config',
        });
    }
    /**
     * Set config (partial)
     * @param requestBody
     * @returns Empty
     * @throws ApiError
     */
    public static setConfig(
        requestBody: PartialConfig,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/config',
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
        });
    }
    /**
     * System info
     * @returns SystemInfo
     * @throws ApiError
     */
    public static getSystemInfo(): CancelablePromise<SystemInfo> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/system',
        });
    }
    /**
     * @returns ShortcutsStatus
     * @throws ApiError
     */
    public static getShortcutsStatus(): CancelablePromise<ShortcutsStatus> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/shortcuts/status',
        });
    }
    /**
     * @returns Empty
     * @throws ApiError
     */
    public static createShortcuts(): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/shortcuts/create',
        });
    }
    /**
     * Logs: retrieve recent service logs
     * @returns string
     * @throws ApiError
     */
    public static getLogs(): CancelablePromise<string> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/logs',
        });
    }
}
