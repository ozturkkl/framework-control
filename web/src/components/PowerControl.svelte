<script lang="ts">
    // @ts-nocheck
    import { onMount, onDestroy } from "svelte";
    import {
        DefaultService,
        OpenAPI,
        type PowerConfig,
        type PowerProfile,
        type PartialConfig,
        type BatteryInfo,
        type PowerCapabilities,
        type PowerState,
    } from "../api";
    import Icon from "@iconify/svelte";
    import { deepMerge } from "../lib/utils";
    import UiSlider from "./UiSlider.svelte";
    import { tooltip } from "../lib/tooltip";

    const POWER_INFO_CONTAINER_CLASS =
        "flex flex-col h-44 my-0.5 px-6 justify-center gap-2";

    let activeProfile: keyof PowerConfig = "ac";
    const ACTIVE_PROFILE_KEY = "fc.power.activeProfile";
    function setActiveProfile(profile: keyof PowerConfig) {
        activeProfile = profile;
        try {
            localStorage.setItem(ACTIVE_PROFILE_KEY, profile);
        } catch (_) {}
    }

    let errorMessage: string | null = null;
    let hasCheckedStatus: boolean = false;
    let infoPoll: ReturnType<typeof setInterval> | null = null;

    // Power method and capabilities
    let powerMethod: string = "none";
    let capabilities: PowerCapabilities | null = null;
    let currentState: PowerState | null = null;

    // Battery info
    let acPresent: boolean | undefined;
    let batteryPct: number | undefined;
    let chargerWatts: number | undefined;
    let chargerRequestedWatts: number | undefined;

    // Unlock high TDP on AC
    let unlockBtn: HTMLButtonElement;
    let unlockTipVisible = false;
    let highTdpUnlocked = false;

    // Power config
    let powerConfig: PowerConfig = {
        ac: {
            tdp_watts: { enabled: false, value: 75 },
            thermal_limit_c: { enabled: false, value: 100 },
            epp_preference: { enabled: false, value: "balance_performance" },
            governor: { enabled: false, value: "schedutil" },
            min_freq_mhz: { enabled: false, value: 1000 },
            max_freq_mhz: { enabled: false, value: 4000 },
        },
        battery: {
            tdp_watts: { enabled: false, value: 60 },
            thermal_limit_c: { enabled: false, value: 90 },
            epp_preference: { enabled: false, value: "balance_power" },
            governor: { enabled: false, value: "powersave" },
            min_freq_mhz: { enabled: false, value: 1000 },
            max_freq_mhz: { enabled: false, value: 3000 },
        },
    };

    $: showControls = hasCheckedStatus && powerMethod !== "none";

    function recomputeHighTdpUnlocked() {
        if (!capabilities?.supports_tdp) return;
        const acVal = powerConfig.ac?.tdp_watts?.value ?? 0;
        const batVal = powerConfig.battery?.tdp_watts?.value ?? 0;
        const acMax = capabilities.tdp_max_watts ?? 120;
        const batMax = Math.min(capabilities.tdp_max_watts ?? 60, 60);
        highTdpUnlocked = acVal > 120 || batVal > batMax;
    }

    async function setPower(
        profile: keyof PowerConfig,
        field: keyof PowerProfile,
        enabled: boolean,
        value: number | string,
    ) {
        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            const patch: PartialConfig = {
                power: {
                    [profile]: {
                        [field]: { enabled, value },
                    },
                },
            };
            await DefaultService.setConfig(auth, patch);
        } catch (e) {
            errorMessage = e instanceof Error ? e.message : String(e);
        }
    }

    function updateChargerWattage(bat: BatteryInfo | undefined) {
        if (
            bat?.charge_input_current_ma != null &&
            bat.charger_voltage_mv != null
        ) {
            chargerWatts =
                (bat.charge_input_current_ma * bat.charger_voltage_mv) /
                1_000_000;
        } else {
            chargerWatts = undefined;
        }
        if (bat?.charger_current_ma != null && bat.charger_voltage_mv != null) {
            chargerRequestedWatts =
                (bat.charger_current_ma * bat.charger_voltage_mv) / 1_000_000;
        } else {
            chargerRequestedWatts = undefined;
        }
    }

    async function pollPower() {
        try {
            const resp = await DefaultService.getPower();

            // Parse new power_control structure
            powerMethod = resp.power_control?.method ?? "none";
            capabilities = resp.power_control?.capabilities ?? null;
            currentState = resp.power_control?.current_state ?? null;

            const bat = resp.battery;
            acPresent = bat?.ac_present;
            batteryPct = bat?.percentage;
            updateChargerWattage(bat);

            // Validate EPP and governor against available options
            validatePowerOptions();
        } catch (_) {
            powerMethod = "none";
            capabilities = null;
            currentState = null;
        } finally {
            hasCheckedStatus = true;
        }
    }

    function validatePowerOptions() {
        if (!capabilities) return;

        // Validate EPP preferences for both profiles
        if (capabilities.available_epp_preferences) {
            for (const profile of ["ac", "battery"] as const) {
                const eppSetting = powerConfig[profile]?.epp_preference;
                if (eppSetting && eppSetting.enabled) {
                    const isValid =
                        capabilities.available_epp_preferences.includes(
                            eppSetting.value,
                        );
                    if (
                        !isValid &&
                        capabilities.available_epp_preferences.length > 0
                    ) {
                        // Reset to first available option
                        eppSetting.value =
                            capabilities.available_epp_preferences[0];
                        setPower(
                            profile,
                            "epp_preference",
                            eppSetting.enabled,
                            eppSetting.value,
                        );
                    }
                }
            }
        }

        // Validate governors for both profiles
        if (capabilities.available_governors) {
            for (const profile of ["ac", "battery"] as const) {
                const govSetting = powerConfig[profile]?.governor;
                if (govSetting && govSetting.enabled) {
                    const isValid = capabilities.available_governors.includes(
                        govSetting.value,
                    );
                    if (
                        !isValid &&
                        capabilities.available_governors.length > 0
                    ) {
                        // Reset to first available option
                        govSetting.value = capabilities.available_governors[0];
                        setPower(
                            profile,
                            "governor",
                            govSetting.enabled,
                            govSetting.value,
                        );
                    }
                }
            }
        }
    }

    onMount(async () => {
        hasCheckedStatus = false;

        try {
            const saved = localStorage.getItem(ACTIVE_PROFILE_KEY);
            if (saved === "ac" || saved === "battery") {
                activeProfile = saved;
            }
        } catch (_) {}

        try {
            const cfg = await DefaultService.getConfig();
            if (cfg.power) {
                powerConfig = deepMerge(
                    powerConfig,
                    cfg.power as PowerConfig,
                    true,
                );
                recomputeHighTdpUnlocked();
            }
        } catch {}

        await pollPower();
        infoPoll = setInterval(pollPower, 2000);
    });

    onDestroy(() => {
        if (infoPoll) clearInterval(infoPoll);
    });

    // Change handlers for each control type
    function onChangeTdp() {
        const tdp = powerConfig[activeProfile]?.tdp_watts;
        if (!tdp) return;
        setPower(activeProfile, "tdp_watts", tdp.enabled, tdp.value);
    }

    function onChangeThermal() {
        const thermal = powerConfig[activeProfile]?.thermal_limit_c;
        if (!thermal) return;
        setPower(
            activeProfile,
            "thermal_limit_c",
            thermal.enabled,
            thermal.value,
        );
    }

    function onChangeEpp() {
        const epp = powerConfig[activeProfile]?.epp_preference;
        if (!epp) return;
        setPower(activeProfile, "epp_preference", epp.enabled, epp.value);
    }

    function onChangeGovernor() {
        const gov = powerConfig[activeProfile]?.governor;
        if (!gov) return;
        setPower(activeProfile, "governor", gov.enabled, gov.value);
    }

    function onChangeFreqMin() {
        const freq = powerConfig[activeProfile]?.min_freq_mhz;
        if (!freq) return;
        setPower(activeProfile, "min_freq_mhz", freq.enabled, freq.value);
    }

    function onChangeFreqMax() {
        const freq = powerConfig[activeProfile]?.max_freq_mhz;
        if (!freq) return;
        setPower(activeProfile, "max_freq_mhz", freq.enabled, freq.value);
    }
</script>

<!-- Preload icons -->
<div aria-hidden="true" class="absolute opacity-0 pointer-events-none -z-10">
    <Icon icon="mdi:power-plug-outline" class="w-3.5 h-3.5" />
    <Icon icon="mdi:battery-outline" class="w-3.5 h-3.5" />
</div>

<!-- Overlay status positioned into the parent header area -->
<div
    class="absolute top-[0.62rem] left-24 right-11 flex items-center justify-between gap-2 text-sm"
>
    {#if showControls}
        <div class="join border border-primary/35">
            <input
                type="radio"
                name="power-profile"
                aria-label="Plugged in"
                class="btn btn-xs join-item"
                value="ac"
                on:change={() => setActiveProfile("ac")}
                checked={activeProfile === "ac"}
            />
            <input
                type="radio"
                name="power-profile"
                aria-label="On battery"
                class="btn btn-xs join-item"
                value="battery"
                on:change={() => setActiveProfile("battery")}
                checked={activeProfile === "battery"}
            />
        </div>

        {#if capabilities?.supports_tdp}
            <div>
                <button
                    class="btn btn-ghost btn-xs"
                    aria-label={highTdpUnlocked
                        ? "Disable high TDP values"
                        : "Unlock higher TDP values"}
                    bind:this={unlockBtn}
                    on:mouseenter={() => (unlockTipVisible = true)}
                    on:mouseleave={() => (unlockTipVisible = false)}
                    on:focus={() => (unlockTipVisible = true)}
                    on:blur={() => (unlockTipVisible = false)}
                    on:click={() => (highTdpUnlocked = !highTdpUnlocked)}
                >
                    <Icon
                        icon={highTdpUnlocked
                            ? "mdi:lock-open-variant-outline"
                            : "mdi:lock-outline"}
                        class="w-3.5 h-3.5"
                    />
                </button>
            </div>

            <div
                use:tooltip={{
                    anchor: unlockBtn,
                    visible: unlockTipVisible,
                    attachGlobalDismiss: false,
                }}
                class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs text-center"
            >
                Unlock higher values for TDP.<br />
                <span class="opacity-90 text-error">USE AT YOUR OWN RISK.</span>
            </div>
        {/if}
    {/if}
</div>

<div class="my-auto">
    <div
        class="bg-base-200 min-w-0 rounded-xl mb-2 py-2 px-3 flex items-center gap-2 text-xs"
    >
        <div
            class="flex flex-wrap items-center gap-x-2 gap-y-1 min-w-0 justify-center mr-auto"
        >
            {#if showControls && currentState}
                {#if currentState.current_watts != null}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <Icon
                            icon="mdi:flash-outline"
                            class="w-4 h-4 text-success"
                        />
                        <span class="tabular-nums text-xs"
                            >{currentState.current_watts.toFixed(1)} W</span
                        >
                    </span>
                {/if}

                {#if currentState.tdp_limit_watts != null}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <span class="text-xs opacity-70">TDP:</span>
                        <span class="tabular-nums text-xs"
                            >{currentState.tdp_limit_watts} W</span
                        >
                    </span>
                {/if}

                {#if currentState.thermal_limit_c != null}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <Icon
                            icon="mdi:thermometer"
                            class="w-4 h-4 text-success"
                        />
                        <span class="tabular-nums text-xs"
                            >{currentState.thermal_limit_c} °C</span
                        >
                    </span>
                {/if}

                {#if currentState.min_freq_mhz != null && currentState.max_freq_mhz != null}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <span class="tabular-nums text-xs"
                            >{(currentState.min_freq_mhz / 1000).toFixed(2)} - {(
                                currentState.max_freq_mhz / 1000
                            ).toFixed(2)} GHz</span
                        >
                    </span>
                {/if}

                {#if currentState.epp_preference}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <span class="text-xs opacity-70"
                            >{currentState.epp_preference}</span
                        >
                    </span>
                {/if}

                {#if currentState.governor}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <span class="text-xs opacity-70"
                            >{currentState.governor}</span
                        >
                    </span>
                {/if}
            {/if}

            {#if acPresent && showControls}
                <span class="opacity-60">•</span>
            {/if}

            {#if acPresent}
                <span class="inline-flex items-center gap-1 whitespace-nowrap">
                    <Icon icon="mdi:power-plug-outline" class="w-3.5 h-3.5" />
                    <span class="tabular-nums text-xs"
                        >{chargerRequestedWatts != null
                            ? Math.round(chargerRequestedWatts)
                            : "—"}/{chargerWatts != null
                            ? Math.round(chargerWatts)
                            : "—"}
                        W</span
                    >
                </span>
            {/if}
        </div>
        <div class="flex gap-x-2 gap-y-1 justify-end whitespace-nowrap">
            <span class={`inline-flex items-center gap-1 whitespace-nowrap`}>
                <Icon
                    icon={acPresent ? "mdi:battery-charging" : "mdi:battery"}
                    class={`w-3.5 h-3.5 ${acPresent ? "animate-pulse" : ""}  ${acPresent ? "text-success" : ""}`}
                />
                <span class="tabular-nums text-xs">{batteryPct ?? "—"}%</span>
            </span>
            <span class="opacity-60">•</span>
            <span
                class={`text-xs opacity-90 ${acPresent ? "text-success" : "text-secondary"}`}
                >{acPresent ? "Plugged in" : "On battery"}</span
            >
        </div>
    </div>

    {#if !hasCheckedStatus}
        <div class={POWER_INFO_CONTAINER_CLASS}>
            <h3 class="text-lg font-bold mb-2 text-center">
                Checking power management…
            </h3>
            <div
                class="flex items-center justify-center gap-2 text-sm opacity-80"
            >
                <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
                <span>Detecting available methods</span>
            </div>
        </div>
    {:else if powerMethod === "none"}
        <div class={POWER_INFO_CONTAINER_CLASS}>
            <h3 class="text-lg font-bold mb-2 text-center mt-2">
                Power controls not available
            </h3>
            <div class="text-sm opacity-80 text-center mb-2">
                No supported power management interface detected. This may be
                due to:
            </div>
            <ul
                class="list-disc text-sm space-y-1 list-inside opacity-70 text-left"
            >
                <li>Unsupported CPU (Intel, or non-AMD on Windows)</li>
                <li>Missing kernel support (Linux needs RAPL/cpufreq)</li>
                <li>RyzenAdj not installed (Windows AMD systems)</li>
            </ul>
        </div>
    {:else}
        <div
            class="grid gap-3 [grid-template-columns:repeat(auto-fit,minmax(18rem,1fr))]"
        >
            <!-- TDP Control (RAPL or RyzenAdj) -->
            {#if capabilities?.supports_tdp && powerConfig[activeProfile]?.tdp_watts}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]?.tdp_watts
                        ?.enabled}
                >
                    <UiSlider
                        label="TDP Limit"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="W"
                        min={capabilities.tdp_min_watts ?? 5}
                        max={highTdpUnlocked
                            ? (capabilities.tdp_max_watts ?? 145)
                            : Math.min(
                                  activeProfile === "ac" ? 120 : 60,
                                  capabilities.tdp_max_watts ?? 120,
                              )}
                        step={1}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].tdp_watts.enabled
                        }
                        capMax={activeProfile === "battery" ? 60 : 120}
                        allowPassingCapMax={highTdpUnlocked}
                        bind:value={powerConfig[activeProfile].tdp_watts.value}
                        on:change={onChangeTdp}
                    />
                </div>
            {/if}

            <!-- Thermal Limit (RyzenAdj) -->
            {#if capabilities?.supports_thermal && powerConfig[activeProfile]?.thermal_limit_c}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.thermal_limit_c?.enabled}
                >
                    <UiSlider
                        label="Thermal Limit"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="°C"
                        min={50}
                        max={100}
                        step={1}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].thermal_limit_c.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].thermal_limit_c.value
                        }
                        on:change={onChangeThermal}
                    />
                </div>
            {/if}

            <!-- AMD P-State EPP -->
            {#if capabilities?.supports_epp && powerConfig[activeProfile]?.epp_preference}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.epp_preference?.enabled}
                >
                    <div class="bg-base-200 rounded-xl p-4">
                        <div class="flex items-center justify-between mb-2">
                            <div class="flex items-center gap-2">
                                <Icon
                                    icon={activeProfile === "ac"
                                        ? "mdi:power-plug-outline"
                                        : "mdi:battery-outline"}
                                    class="w-4 h-4"
                                />
                                <span class="font-medium text-sm"
                                    >Energy Preference</span
                                >
                            </div>
                            <input
                                type="checkbox"
                                class="toggle toggle-sm toggle-primary"
                                bind:checked={
                                    powerConfig[activeProfile].epp_preference
                                        .enabled
                                }
                                on:change={onChangeEpp}
                            />
                        </div>
                        <select
                            class="select select-sm select-bordered w-full"
                            bind:value={
                                powerConfig[activeProfile].epp_preference.value
                            }
                            on:change={onChangeEpp}
                            disabled={!powerConfig[activeProfile].epp_preference
                                .enabled}
                        >
                            {#each capabilities.available_epp_preferences ?? ["power", "balance_power", "balance_performance", "performance"] as pref}
                                <option value={pref}
                                    >{pref.replace(/_/g, " ")}</option
                                >
                            {/each}
                        </select>
                    </div>
                </div>
            {/if}

            <!-- cpufreq Governor -->
            {#if capabilities?.supports_governor && powerConfig[activeProfile]?.governor}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]?.governor
                        ?.enabled}
                >
                    <div class="bg-base-200 rounded-xl p-4">
                        <div class="flex items-center justify-between mb-2">
                            <div class="flex items-center gap-2">
                                <Icon
                                    icon={activeProfile === "ac"
                                        ? "mdi:power-plug-outline"
                                        : "mdi:battery-outline"}
                                    class="w-4 h-4"
                                />
                                <span class="font-medium text-sm"
                                    >CPU Governor</span
                                >
                            </div>
                            <input
                                type="checkbox"
                                class="toggle toggle-sm toggle-primary"
                                bind:checked={
                                    powerConfig[activeProfile].governor.enabled
                                }
                                on:change={onChangeGovernor}
                            />
                        </div>
                        <select
                            class="select select-sm select-bordered w-full"
                            bind:value={
                                powerConfig[activeProfile].governor.value
                            }
                            on:change={onChangeGovernor}
                            disabled={!powerConfig[activeProfile].governor
                                .enabled}
                        >
                            {#each capabilities.available_governors ?? ["powersave", "schedutil", "performance"] as gov}
                                <option value={gov}>{gov}</option>
                            {/each}
                        </select>
                    </div>
                </div>
            {/if}

            <!-- Frequency Limits (cpufreq) -->
            {#if capabilities?.supports_frequency_limits && powerConfig[activeProfile]?.min_freq_mhz}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.min_freq_mhz?.enabled}
                >
                    <UiSlider
                        label="Min Frequency"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="MHz"
                        min={capabilities.frequency_min_mhz ?? 400}
                        max={capabilities.frequency_max_mhz ?? 5000}
                        step={100}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].min_freq_mhz.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].min_freq_mhz.value
                        }
                        on:change={onChangeFreqMin}
                    />
                </div>
            {/if}

            {#if capabilities?.supports_frequency_limits && powerConfig[activeProfile]?.max_freq_mhz}
                <div
                    class="transition-transform duration-100"
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.max_freq_mhz?.enabled}
                >
                    <UiSlider
                        label="Max Frequency"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="MHz"
                        min={capabilities.frequency_min_mhz ?? 400}
                        max={capabilities.frequency_max_mhz ?? 5000}
                        step={100}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].max_freq_mhz.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].max_freq_mhz.value
                        }
                        on:change={onChangeFreqMax}
                    />
                </div>
            {/if}
        </div>
    {/if}

    {#if errorMessage}
        <div class="text-xs text-error mt-2">{errorMessage}</div>
    {/if}
</div>
