/**
 * Platform detection utilities
 */

/**
 * Get the platform string from navigator, trying modern API first
 */
function getPlatformString(): string {
    if (typeof navigator === "undefined") {
        return "";
    }

    // Try userAgentData.platform first (newer Chromium API)
    const nav = navigator as Navigator & {
        userAgentData?: { platform?: string };
    };
    if (nav.userAgentData?.platform) {
        return nav.userAgentData.platform;
    }

    // Fall back to navigator.platform
    if (navigator.platform) {
        return navigator.platform;
    }

    // Last resort: parse userAgent
    if (navigator.userAgent) {
        return navigator.userAgent;
    }

    return "";
}

/**
 * Detect if running on Linux
 */
export function isLinux(): boolean {
    return getPlatformString().toLowerCase().includes("linux");
}

/**
 * Detect if running on Windows
 */
export function isWindows(): boolean {
    const platform = getPlatformString().toLowerCase();
    return platform.includes("win");
}