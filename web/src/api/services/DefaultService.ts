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
     * @param authorization
     * @returns Empty
     * @throws ApiError
     */
    public static installRyzenadj(
        authorization: string,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/ryzenadj/install',
            headers: {
                'Authorization': authorization,
            },
        });
    }
    /**
     * RyzenAdj: uninstall and remove any downloaded artifacts (Windows only)
     * @param authorization
     * @returns Empty
     * @throws ApiError
     */
    public static uninstallRyzenadj(
        authorization: string,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/ryzenadj/uninstall',
            headers: {
                'Authorization': authorization,
            },
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
     * @param authorization
     * @param requestBody
     * @returns Empty
     * @throws ApiError
     */
    public static applyUpdate(
        authorization: string,
        requestBody: any,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/update/apply',
            headers: {
                'Authorization': authorization,
            },
            body: requestBody,
            mediaType: 'application/json; charset=utf-8',
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
     * @param authorization
     * @param requestBody
     * @returns Empty
     * @throws ApiError
     */
    public static setConfig(
        authorization: string,
        requestBody: PartialConfig,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/config',
            headers: {
                'Authorization': authorization,
            },
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
     * @param authorization
     * @returns Empty
     * @throws ApiError
     */
    public static createShortcuts(
        authorization: string,
    ): CancelablePromise<Empty> {
        return __request(OpenAPI, {
            method: 'POST',
            url: '/shortcuts/create',
            headers: {
                'Authorization': authorization,
            },
        });
    }
    /**
     * Logs: retrieve recent service logs
     * @param authorization
     * @returns string
     * @throws ApiError
     */
    public static getLogs(
        authorization: string,
    ): CancelablePromise<string> {
        return __request(OpenAPI, {
            method: 'GET',
            url: '/logs',
            headers: {
                'Authorization': authorization,
            },
        });
    }
}
