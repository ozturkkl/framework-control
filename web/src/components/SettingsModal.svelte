<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { onMount } from "svelte";
    import Icon from "@iconify/svelte";
    import ShortcutInstaller from "./ShortcutInstaller.svelte";
    import { DefaultService, OpenAPI, type PartialConfig } from "../api";
    import { gtSemver } from "../lib/semver";
    import { listAvailableDaisyUIThemes } from "../lib/themes";
    import { isLinux } from "../lib/platform";
    const dispatch = createEventDispatcher();
    function close() {
        dispatch("close");
    }

    let currentVersion: string;
    let latestVersion: string;
    let newVersionAvailable: boolean = false;

    let autoInstall: boolean = false;
    let updatesPaused: boolean =
        localStorage.getItem("fc_updates_paused") === "1";

    let isChecking: boolean = false;
    let isPaused: boolean = false;
    let applying = false;
    let errorMessage: string | null = null;

    // Theme handling (DaisyUI)
    let themeOptions: string[] = listAvailableDaisyUIThemes();
    let theme: string = localStorage?.getItem("fc_theme") ?? "light";
    function onThemeChange(event: Event) {
        const target = event.currentTarget as HTMLSelectElement;
        theme = target?.value ?? theme;
        localStorage.setItem("fc_theme", theme);
        document.documentElement.setAttribute("data-theme", theme);
        // Persist to backend for cross-client consistency
        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            const body: PartialConfig = {
                ui: { theme },
            };
            DefaultService.setConfig(auth, body);
        } catch {
            // non-fatal; leave localStorage applied
        }
    }

    async function checkUpdate() {
        try {
            isChecking = true;
            const data = await DefaultService.checkUpdate();
            currentVersion = data.current_version?.toString().trim();
            latestVersion = data.latest_version?.toString().trim();
            console.debug("[SettingsModal] checkUpdate result", {
                currentVersion,
                latestVersion,
                updatesPaused,
            });
            errorMessage = null;
        } catch {
            errorMessage = "Failed to check for updates!";
        } finally {
            isChecking = false;
        }
    }

    async function applyUpdate() {
        applying = true;
        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            await DefaultService.applyUpdate(auth, {});
            await new Promise((resolve) => setTimeout(resolve, 5000));
            errorMessage = null;
            if (typeof window !== "undefined") {
                window.location.reload();
            }
        } catch (e) {
            errorMessage = "Failed to apply update!";
        } finally {
            applying = false;
            await checkUpdate();
        }
    }

    async function loadBackendUpdatePrefs() {
        try {
            const cfg = await DefaultService.getConfig();
            autoInstall = !!cfg?.updates?.auto_install;
            errorMessage = null;
        } catch {
            autoInstall = false;
            errorMessage = "Failed to load auto update preference!";
        }
    }

    async function onToggleAutoInstall(event: Event) {
        const target = event.currentTarget as HTMLInputElement | null;
        const nextValue = target?.checked ?? autoInstall;
        const previousValue = !nextValue;
        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            const body: PartialConfig = {
                updates: { auto_install: nextValue },
            } as PartialConfig;
            await DefaultService.setConfig(auth, body);
            errorMessage = null;
            // If enabling auto-install, reuse existing applyUpdate() and then re-check
            if (nextValue && newVersionAvailable) {
                await applyUpdate();
            }
        } catch {
            // Revert UI state if backend update fails
            autoInstall = previousValue;
            errorMessage = "Failed to set auto update preference!";
        }
    }

    async function pauseUpdates() {
        updatesPaused = true;
        localStorage.setItem("fc_updates_paused", "1");
        if (typeof window !== "undefined") {
            window.dispatchEvent(new Event("fc_updates_paused_changed"));
        }
        // If auto-install was enabled, disable it and inform backend
        if (autoInstall) {
            autoInstall = false;
            try {
                const auth = `Bearer ${OpenAPI.TOKEN}`;
                const body: PartialConfig = {
                    updates: { auto_install: false },
                } as PartialConfig;
                await DefaultService.setConfig(auth, body);
                errorMessage = null;
            } catch {
                autoInstall = true;
                errorMessage = "Failed to pause updates!";
            }
        }
    }

    async function unpauseUpdates() {
        updatesPaused = false;
        localStorage.removeItem("fc_updates_paused");
        if (typeof window !== "undefined") {
            window.dispatchEvent(new Event("fc_updates_paused_changed"));
        }
        console.debug("[SettingsModal] unpaused", { updatesPaused });
        await checkUpdate();
    }

    // auto-check on mount and load backend prefs
    onMount(() => {
        checkUpdate();
        loadBackendUpdatePrefs();
    });

    $: newVersionAvailable =
        currentVersion && latestVersion
            ? gtSemver(latestVersion, currentVersion)
            : false;

    $: isPaused = updatesPaused;
    $: console.debug("[SettingsModal] reactive", {
        isPaused,
        newVersionAvailable,
        currentVersion,
        latestVersion,
        isChecking,
    });
</script>

<div class="modal modal-open">
    <div class="modal-box max-w-2xl">
        <h3 class="font-bold text-lg">Settings</h3>
        <div class="divider my-2"></div>
        <div class="space-y-4">
            <section
                class="flex flex-col md:flex-row items-start justify-between gap-4"
            >
                <div class="flex-1 space-y-1">
                    <div class="flex items-center gap-2">
                        <h4 class="font-semibold">Updates</h4>
                        {#if isPaused}
                            <div class="badge badge-neutral badge-sm gap-1">
                                <Icon icon="mdi:pause-circle" class="w-3 h-3" />
                                Paused Updates
                            </div>

                            <p class="text-xs opacity-70">
                                Version {currentVersion}
                            </p>
                        {:else if newVersionAvailable}
                            <div class="badge badge-warning badge-sm gap-1">
                                <Icon icon="mdi:package-up" class="w-3 h-3" />
                                Available
                            </div>

                            <p class="text-xs opacity-70">
                                Version {currentVersion} → {latestVersion}
                            </p>
                        {:else if currentVersion && latestVersion}
                            <div class="badge badge-success badge-sm gap-1">
                                <Icon icon="mdi:check-circle" class="w-3 h-3" />
                                Up to date
                            </div>
                            <p class="text-xs opacity-70">
                                Version {currentVersion}
                            </p>
                        {:else}
                            <div class="badge badge-ghost badge-sm gap-1">
                                <Icon
                                    icon="mdi:loading animate-spin"
                                    class="w-3 h-3"
                                />
                                Checking...
                            </div>
                        {/if}
                    </div>
                    <div class="text-xs opacity-70 space-y-1 mt-1">
                        {#if isPaused}
                            <div>You've paused update notifications.</div>
                            <div>
                                Click "Unpause" to resume update notifications.
                            </div>
                        {:else if newVersionAvailable}
                            <div>
                                Click "Install" to download and install the
                                latest version.
                            </div>
                            <div>Your settings will be preserved.</div>
                        {:else if currentVersion && latestVersion}
                            <div>You're on the latest version. 🥳</div>
                        {/if}
                    </div>
                </div>
                <div class="flex flex-col items-end gap-2 md:w-auto">
                    <div class="flex items-center gap-2">
                        {#if isPaused}
                            <button
                                class="btn btn-sm btn-primary"
                                on:click={unpauseUpdates}
                            >
                                Unpause
                            </button>
                        {:else if newVersionAvailable}
                            <button
                                class="btn btn-sm btn-ghost"
                                on:click={pauseUpdates}
                                disabled={applying}>Pause</button
                            >
                            <button
                                class="btn btn-sm btn-primary"
                                on:click={() => {
                                    if (!applying) applyUpdate();
                                }}
                            >
                                {#if applying}
                                    <Icon
                                        icon="mdi:loading"
                                        class="w-4 h-4 animate-spin"
                                    />
                                    Installing
                                {:else}
                                    <Icon icon="mdi:download" class="w-4 h-4" />
                                    Install
                                {/if}
                            </button>
                            {#if errorMessage}
                                <div class="text-xs opacity-70">
                                    {errorMessage}
                                </div>
                            {/if}
                        {/if}
                    </div>
                    <div class="text-right space-y-1">
                        <label
                            class="label cursor-pointer justify-start md:justify-end gap-2 text-xs"
                        >
                            <span class="label-text" class:opacity-50={isPaused}
                                >Automatically keep up to date</span
                            >
                            <input
                                type="checkbox"
                                class="toggle toggle-xs"
                                bind:checked={autoInstall}
                                disabled={isPaused || applying}
                                on:change={onToggleAutoInstall}
                            />
                        </label>
                    </div>
                </div>
            </section>
            <div class="divider opacity-80"></div>
            <section class="flex items-center justify-between gap-4">
                <div>
                    <h4 class="font-semibold">Theme</h4>
                    <p class="text-xs opacity-70">Choose a look and feel</p>
                </div>
                <div class="flex items-center gap-2">
                    <select
                        class="select select-sm"
                        bind:value={theme}
                        on:change={onThemeChange}
                        aria-label="Select theme"
                    >
                        {#each themeOptions as t}
                            <option value={t}
                                >{t.charAt(0).toUpperCase() +
                                    t.slice(1)}</option
                            >
                        {/each}
                    </select>
                </div>
            </section>
            <div class="divider opacity-80"></div>
            <section class="flex items-center justify-between gap-4">
                <div>
                    <h4 class="font-semibold">
                        {isLinux() ? "Desktop Entry" : "Desktop Shortcuts"}
                    </h4>
                    <p class="text-xs opacity-70">
                        {isLinux()
                            ? "Create application launcher entry for quick access"
                            : "Create Start Menu and Desktop shortcuts for quick access"}
                    </p>
                </div>
                <ShortcutInstaller />
            </section>
        </div>
        <div class="modal-action">
            <button class="btn" on:click={close}>Close</button>
        </div>
    </div>
    <button class="modal-backdrop" on:click={close} aria-label="Close settings"
    ></button>
</div>
