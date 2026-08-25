<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import {
        DefaultService,
        type PowerConfig,
        type PowerProfile,
        type BatteryInfo,
        type PowerCapabilities,
        type PowerState,
    } from "../api";
    import Icon from "@iconify/svelte";
    import { deepMerge } from "../lib/utils";
    import { followConfig, patch } from "../lib/config";
    import UiControlCard from "./UiControlCard.svelte";
    import {
        PANEL_HEADER_OVERLAY_CLASS,
        PANEL_HEADER_TOGGLE_CLASS,
    } from "./Panel.svelte";
    import { tooltip } from "../lib/tooltip";
    import { isWindows as getIsWindows } from "../lib/platform";

    const POWER_INFO_CONTAINER_CLASS =
        "flex flex-col h-44 my-0.5 px-6 justify-center gap-2";
    const POWER_CONTROLS_GRID_CLASS =
        "grid flex-1 min-h-0 gap-3 pb-1 auto-rows-[minmax(min-content,1fr)] [grid-template-columns:repeat(auto-fit,minmax(18rem,1fr))]";
    const POWER_CONTROL_WRAP_CLASS =
        "h-full min-h-0 transition-transform duration-100";
    const isWindows = getIsWindows();

    let activeProfile: keyof PowerConfig = "ac";
    const ACTIVE_PROFILE_KEY = "fc.power.activeProfile";
    const POWER_PROFILES: { value: keyof PowerConfig; label: string }[] = [
        { value: "ac", label: "Plugged in" },
        { value: "battery", label: "On battery" },
    ];

    function setActiveProfile(profile: keyof PowerConfig) {
        activeProfile = profile;
        try {
            localStorage.setItem(ACTIVE_PROFILE_KEY, profile);
        } catch (_) {}
    }

    let installingRyzenAdj = false;
    let uninstallingRyzenAdj = false;
    let errorMessage: string | null = null;
    let infoPoll: ReturnType<typeof setInterval> | null = null;
    let agreed = false;
    let hasCheckedStatus: boolean = false;

    function defaultPowerConfig(): PowerConfig {
        return {
            ac: {
                tdp_watts: { enabled: false, value: 75 },
                thermal_limit_c: { enabled: false, value: 90 },
                epp_preference: { enabled: false, value: "" },
                governor: { enabled: false, value: "" },
                min_freq_mhz: { enabled: false, value: 1000 },
                max_freq_mhz: { enabled: false, value: 4000 },
            },
            battery: {
                tdp_watts: { enabled: false, value: 60 },
                thermal_limit_c: { enabled: false, value: 90 },
                epp_preference: { enabled: false, value: "" },
                governor: { enabled: false, value: "" },
                min_freq_mhz: { enabled: false, value: 1000 },
                max_freq_mhz: { enabled: false, value: 3000 },
            },
        };
    }

    let powerConfig: PowerConfig = defaultPowerConfig();

    // Capabilities + current state reported by the backend
    let capabilities: PowerCapabilities | null = null;
    let currentState: PowerState | null = null;

    // Battery info
    let acPresent: boolean | undefined;
    let batteryPct: number | undefined;
    let chargerWatts: number | undefined;
    let chargerRequestedWatts: number | undefined;

    // Unlock high TDP on AC
    let removeBtn: HTMLButtonElement;
    let removeTipVisible = false;
    let unlockBtn: HTMLButtonElement;
    let unlockTipVisible = false;
    let highTdpUnlocked = false;

    // Frequency-limits mismatch warning (one profile applies limits, the other noops)
    let freqWarningBtn: HTMLButtonElement;
    let freqWarningTipVisible = false;

    $: hasAnyPowerCapability =
        !!capabilities &&
        (capabilities.supports_tdp ||
            capabilities.supports_thermal ||
            capabilities.supports_epp ||
            capabilities.supports_governor ||
            capabilities.supports_frequency_limits);

    $: showControls = hasCheckedStatus && hasAnyPowerCapability;

    $: hasFreqLimitsMismatchWarning = (() => {
        if (!capabilities?.supports_frequency_limits) return false;
        const acMin = !!powerConfig?.ac?.min_freq_mhz?.enabled;
        const acMax = !!powerConfig?.ac?.max_freq_mhz?.enabled;
        const batMin = !!powerConfig?.battery?.min_freq_mhz?.enabled;
        const batMax = !!powerConfig?.battery?.max_freq_mhz?.enabled;
        const acAny = acMin || acMax;
        const batAny = batMin || batMax;
        return (acAny && !batMin && !batMax) || (batAny && !acMin && !acMax);
    })();

    type StatusMetric = {
        id: string;
        icon?: string;
        iconClass?: string;
        label?: string;
        value?: string;
    };

    $: statusMetrics = ((): StatusMetric[] => {
        const metrics: StatusMetric[] = [];
        if (showControls && currentState) {
            if (currentState.tdp_limit_watts != null) {
                const tdp = Number(currentState.tdp_limit_watts);
                metrics.push({
                    id: "tdp",
                    icon: "mdi:flash-outline",
                    iconClass: `w-4 h-4 ${tdp > 95 ? "brightness-200" : tdp > 60 ? "brightness-150" : "brightness-100"} text-success`,
                    label: "TDP:",
                    value: `${currentState.tdp_limit_watts} W`,
                });
            }
            if (currentState.thermal_limit_c != null) {
                const thermal = Number(currentState.thermal_limit_c);
                metrics.push({
                    id: "thermal",
                    icon: "mdi:thermometer",
                    iconClass: `w-4 h-4 ${thermal > 95 ? "text-error" : thermal > 90 ? "text-warning" : "text-success"}`,
                    value: `${currentState.thermal_limit_c} °C`,
                });
            }
            if (
                currentState.min_freq_mhz != null &&
                currentState.max_freq_mhz != null
            ) {
                metrics.push({
                    id: "freq",
                    value: `${(currentState.min_freq_mhz / 1000).toFixed(2)} - ${(
                        currentState.max_freq_mhz / 1000
                    ).toFixed(2)} GHz`,
                });
            }
            if (currentState.epp_preference) {
                metrics.push({
                    id: "epp",
                    label: currentState.epp_preference,
                });
            }
            if (currentState.governor) {
                metrics.push({
                    id: "governor",
                    label: currentState.governor,
                });
            }
        }
        if (acPresent) {
            metrics.push({
                id: "charger",
                icon: "mdi:power-plug-outline",
                iconClass: "w-3.5 h-3.5",
                value: `${chargerRequestedWatts != null ? Math.round(chargerRequestedWatts) : "—"}/${chargerWatts != null ? Math.round(chargerWatts) : "—"} W`,
            });
        }
        return metrics;
    })();

    type BatterySummaryItem = {
        id: string;
        icon?: string;
        iconClass?: string;
        value?: string;
        text?: string;
        textClass?: string;
    };

    $: batterySummary = ((): BatterySummaryItem[] => [
        {
            id: "pct",
            icon: acPresent ? "mdi:battery-charging" : "mdi:battery",
            iconClass: `w-3.5 h-3.5 ${acPresent ? "animate-pulse" : ""}  ${acPresent ? "text-success" : ""}`,
            value: `${batteryPct ?? "—"}%`,
        },
        {
            id: "status",
            text: acPresent ? "Plugged in" : "On battery",
            textClass: `text-xs opacity-90 ${acPresent ? "text-success" : "text-secondary"}`,
        },
    ])();

    function recomputeHighTdpUnlocked() {
        if (!capabilities?.supports_tdp) return;
        const acVal = powerConfig.ac?.tdp_watts?.value ?? 0;
        const batVal = powerConfig.battery?.tdp_watts?.value ?? 0;
        highTdpUnlocked = acVal > 120 || batVal > 60;
    }

    function applyPowerConfig(pow: PowerConfig) {
        powerConfig = deepMerge(defaultPowerConfig(), pow, true);
        recomputeHighTdpUnlocked();
    }

    onDestroy(followConfig({ select: (c) => c.power, apply: applyPowerConfig }));

    async function setPower(
        profile: keyof PowerConfig,
        field: keyof PowerProfile,
        enabled: boolean,
        value: number | string,
    ) {
        try {
            await patch({
                power: {
                    [profile]: {
                        [field]: { enabled, value },
                    },
                },
            });
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

            // Parse power_control structure (capability-driven UI; no method string)
            capabilities = resp.power_control?.capabilities ?? null;
            currentState = resp.power_control?.current_state ?? null;

            const bat = resp.battery;
            acPresent = bat?.ac_present;
            batteryPct = bat?.percentage;
            updateChargerWattage(bat);

        } catch (_) {
            capabilities = null;
            currentState = null;
        } finally {
            hasCheckedStatus = true;
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
        await pollPower();
        infoPoll = setInterval(pollPower, 2000);
    });
    onDestroy(() => {
        if (infoPoll) clearInterval(infoPoll);
    });

    async function installRyzenAdj() {
        if (!isWindows || !agreed) return;
        installingRyzenAdj = true;
        errorMessage = null;
        try {
            await DefaultService.installRyzenadj();
            for (let i = 0; i < 5; i++) {
                await pollPower();
                if (hasAnyPowerCapability) break;
                await new Promise((resolve) => setTimeout(resolve, 1000));
            }
        } catch (e) {
            errorMessage = "Failed to install, check your antivirus settings!";
        } finally {
            installingRyzenAdj = false;
        }
    }

    async function uninstallRyzenAdj() {
        uninstallingRyzenAdj = true;
        errorMessage = null;
        try {
            await DefaultService.uninstallRyzenadj();
            await pollPower();
        } catch (e) {
            errorMessage = e instanceof Error ? e.message : String(e);
        } finally {
            uninstallingRyzenAdj = false;
        }
    }

    // Change handlers for each control type
    function onChangeProfileField(field: keyof PowerProfile) {
        const setting = powerConfig[activeProfile]?.[field];
        if (!setting) return;
        setPower(activeProfile, field, setting.enabled, setting.value);
    }

    async function setFreqLimits(
        profile: keyof PowerConfig,
        minVal: number,
        minEnabled: boolean,
        maxVal: number,
        maxEnabled: boolean,
    ) {
        try {
            await patch({
                power: {
                    [profile]: {
                        min_freq_mhz: { enabled: minEnabled, value: minVal },
                        max_freq_mhz: { enabled: maxEnabled, value: maxVal },
                    },
                },
            });
        } catch (e) {
            errorMessage = e instanceof Error ? e.message : String(e);
        }
    }

    function onChangeFreqLimits() {
        const minSetting = powerConfig[activeProfile]?.min_freq_mhz;
        const maxSetting = powerConfig[activeProfile]?.max_freq_mhz;
        if (!minSetting || !maxSetting) return;
        setFreqLimits(
            activeProfile,
            minSetting.value,
            minSetting.enabled,
            maxSetting.value,
            maxSetting.enabled,
        );
    }
</script>

<div class={PANEL_HEADER_OVERLAY_CLASS}>
    <div
        class={PANEL_HEADER_TOGGLE_CLASS}
        role="radiogroup"
        aria-label="Power profile"
    >
        {#each POWER_PROFILES as opt (opt.value)}
            <input
                type="radio"
                name="power-profile"
                aria-label={opt.label}
                class="btn btn-xs join-item"
                value={opt.value}
                checked={activeProfile === opt.value}
                on:change={() => setActiveProfile(opt.value)}
            />
        {/each}
    </div>
</div>

<!-- Preload icons -->
<div aria-hidden="true" class="absolute opacity-0 pointer-events-none -z-10">
    <Icon icon="mdi:power-plug-outline" class="w-3.5 h-3.5" />
    <Icon icon="mdi:battery-outline" class="w-3.5 h-3.5" />
</div>

<div class="min-h-0 flex flex-col flex-1">
    <div
        class="bg-base-200 min-w-0 rounded-xl mb-2 py-2 px-3 flex flex-wrap items-center gap-2 text-xs shrink-0"
    >
        <div
            class="flex flex-wrap items-center gap-x-2 gap-y-1 min-w-0 justify-center mr-auto"
        >
            {#each statusMetrics as metric, i (metric.id)}
                {#if i > 0}
                    <span class="opacity-60">•</span>
                {/if}
                <span
                    class="inline-flex items-center gap-1 whitespace-nowrap"
                >
                    {#if metric.icon}
                        <Icon icon={metric.icon} class={metric.iconClass} />
                    {/if}
                    {#if metric.label}
                        <span class="text-xs opacity-70">{metric.label}</span>
                    {/if}
                    {#if metric.value}
                        <span class="tabular-nums text-xs">{metric.value}</span>
                    {/if}
                </span>
            {/each}
        </div>
        {#if showControls && (hasFreqLimitsMismatchWarning || capabilities?.supports_tdp)}
            <div class="flex items-center">
                {#if hasFreqLimitsMismatchWarning}
                    <div class="relative">
                        <button
                            class="btn btn-ghost btn-xs text-warning"
                            aria-label="Frequency limits warning"
                            bind:this={freqWarningBtn}
                            on:mouseenter={() =>
                                (freqWarningTipVisible = true)}
                            on:mouseleave={() =>
                                (freqWarningTipVisible = false)}
                            on:focus={() => (freqWarningTipVisible = true)}
                            on:blur={() => (freqWarningTipVisible = false)}
                        >
                            <Icon icon="mdi:alert-outline" class="w-4 h-4" />
                        </button>

                        <div
                            use:tooltip={{
                                anchor: freqWarningBtn,
                                visible: freqWarningTipVisible,
                                attachGlobalDismiss: false,
                            }}
                            class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs w-64 text-center"
                        >
                            One profile applies CPU frequency limits, but the
                            other profile has them disabled. When switching to
                            the disabled profile, Framework Control won’t reset
                            touch the limits, so they may remain active until
                            something else changes them (reboot/OS power
                            daemon/etc).
                        </div>
                    </div>
                {/if}
                {#if capabilities?.supports_tdp}
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
                    {#if isWindows}
                        <button
                            class="btn btn-ghost btn-xs"
                            aria-label="Remove helper"
                            bind:this={removeBtn}
                            on:mouseenter={() => (removeTipVisible = true)}
                            on:mouseleave={() => (removeTipVisible = false)}
                            on:focus={() => (removeTipVisible = true)}
                            on:blur={() => (removeTipVisible = false)}
                            on:click={uninstallRyzenAdj}
                            disabled={uninstallingRyzenAdj}
                        >
                            {#if uninstallingRyzenAdj}
                                <Icon
                                    icon="mdi:loading"
                                    class="w-3.5 h-3.5 animate-spin"
                                />
                            {:else}
                                <Icon
                                    icon="mdi:trash-can-outline"
                                    class="w-3.5 h-3.5"
                                />
                            {/if}
                        </button>
                    {/if}
                {/if}
            </div>
        {/if}
        <div
            class="flex gap-x-2 gap-y-1 justify-end whitespace-nowrap ml-auto"
        >
            {#each batterySummary as item, i (item.id)}
                {#if i > 0}
                    <span class="opacity-60">•</span>
                {/if}
                {#if item.icon}
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <Icon icon={item.icon} class={item.iconClass} />
                        <span class="tabular-nums text-xs">{item.value}</span>
                    </span>
                {:else}
                    <span class={item.textClass}>{item.text}</span>
                {/if}
            {/each}
        </div>
    </div>
    {#if showControls && capabilities?.supports_tdp}
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
        {#if isWindows}
            <div
                use:tooltip={{
                    anchor: removeBtn,
                    visible: removeTipVisible,
                    attachGlobalDismiss: false,
                }}
                class="pointer-events-none bg-base-100 px-2 py-1 rounded border border-base-300 shadow text-xs w-60 text-center"
            >
                Remove the RyzenAdj helper. You can reinstall later from
                here.
            </div>
        {/if}
    {/if}

    {#if !hasCheckedStatus}
        <div class={POWER_INFO_CONTAINER_CLASS}>
            <h3 class="text-lg font-bold mb-2 text-center">
                Checking requirements…
            </h3>
            <div
                class="flex items-center justify-center gap-2 text-sm opacity-80"
            >
                <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
                <span>Detecting current power helper status</span>
            </div>
        </div>
    {:else if isWindows && !hasAnyPowerCapability}
        <div class={POWER_INFO_CONTAINER_CLASS}>
            <h3 class="text-lg font-bold mb-2 text-center">
                Enable power controls
            </h3>
            <ul class="list-disc text-sm space-y-1 list-inside opacity-80">
                <li>
                    This requires a small helper <a
                        href="https://github.com/FlyGoat/RyzenAdj"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="btn-link px-0">RyzenAdj</a
                    > to be installed.
                </li>
                <li>May trigger antivirus warnings on your system.</li>
                <li>
                    Adjusting power settings can cause instability and crashes
                    and may even (though rarely) damage your hardware. We take
                    no responsibility!
                </li>
            </ul>
            <div class="mt-1 flex items-center justify-between">
                <label class="label cursor-pointer justify-start gap-2">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-sm"
                        bind:checked={agreed}
                    />
                    <span class="label-text text-sm"
                        >I agree to the above and <span class="text-primary"
                            >understand the risks!</span
                        ></span
                    >
                </label>
                <button
                    class="btn btn-primary btn-sm"
                    disabled={!agreed || installingRyzenAdj}
                    on:click={installRyzenAdj}
                >
                    {#if installingRyzenAdj}
                        <Icon icon="mdi:loading" class="w-4 h-4 animate-spin" />
                        Installing...
                    {:else}
                        <Icon icon="mdi:download-outline" class="w-4 h-4" />
                        Install
                    {/if}
                </button>
            </div>
        </div>
    {:else if !hasAnyPowerCapability}
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
                {#if isWindows}
                    <li>RyzenAdj not installed (Windows AMD systems)</li>
                {/if}
            </ul>
        </div>
    {:else}
        <div class={POWER_CONTROLS_GRID_CLASS}>
            <!-- TDP Control (RAPL or RyzenAdj) -->
            {#if capabilities?.supports_tdp && powerConfig[activeProfile]?.tdp_watts}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]?.tdp_watts
                        ?.enabled}
                >
                    <UiControlCard
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
                        on:change={() => onChangeProfileField("tdp_watts")}
                    />
                </div>
            {/if}

            <!-- Thermal Limit (RyzenAdj) -->
            {#if capabilities?.supports_thermal && powerConfig[activeProfile]?.thermal_limit_c}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.thermal_limit_c?.enabled}
                >
                    <UiControlCard
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
                        on:change={() =>
                            onChangeProfileField("thermal_limit_c")}
                    />
                </div>
            {/if}

            <!-- AMD P-State EPP -->
            {#if capabilities?.supports_epp && powerConfig[activeProfile]?.epp_preference}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.epp_preference?.enabled}
                >
                    <UiControlCard
                        label="Energy Preference"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        variant="select"
                        options={capabilities.available_epp_preferences ?? [
                            "power",
                            "balance_power",
                            "balance_performance",
                            "performance",
                        ]}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].epp_preference.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].epp_preference.value
                        }
                        on:change={() => onChangeProfileField("epp_preference")}
                    />
                </div>
            {/if}

            <!-- cpufreq Governor -->
            {#if capabilities?.supports_governor && powerConfig[activeProfile]?.governor}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]?.governor
                        ?.enabled}
                >
                    <UiControlCard
                        label="CPU Governor"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        variant="select"
                        options={capabilities.available_governors ?? [
                            "powersave",
                            "schedutil",
                            "performance",
                        ]}
                        hasEnabled={true}
                        bind:enabled={
                            powerConfig[activeProfile].governor.enabled
                        }
                        bind:value={powerConfig[activeProfile].governor.value}
                        on:change={() => onChangeProfileField("governor")}
                    />
                </div>
            {/if}

            <!-- Frequency Limits (cpufreq) -->
            {#if capabilities?.supports_frequency_limits && powerConfig[activeProfile]?.min_freq_mhz}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.min_freq_mhz?.enabled}
                >
                    <UiControlCard
                        label="Min Frequency"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="MHz"
                        min={capabilities.frequency_min_mhz ?? 400}
                        max={capabilities.frequency_max_mhz ?? 5000}
                        step={100}
                        hasEnabled={true}
                        capMax={powerConfig[activeProfile].max_freq_mhz
                            ?.value ?? null}
                        bind:enabled={
                            powerConfig[activeProfile].min_freq_mhz.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].min_freq_mhz.value
                        }
                        on:change={onChangeFreqLimits}
                    />
                </div>
            {/if}

            {#if capabilities?.supports_frequency_limits && powerConfig[activeProfile]?.max_freq_mhz}
                <div
                    class={POWER_CONTROL_WRAP_CLASS}
                    class:scale-[0.985]={!powerConfig[activeProfile]
                        ?.max_freq_mhz?.enabled}
                >
                    <UiControlCard
                        label="Max Frequency"
                        icon={activeProfile === "ac"
                            ? "mdi:power-plug-outline"
                            : "mdi:battery-outline"}
                        unit="MHz"
                        min={capabilities.frequency_min_mhz ?? 400}
                        max={capabilities.frequency_max_mhz ?? 5000}
                        step={100}
                        hasEnabled={true}
                        capMin={powerConfig[activeProfile].min_freq_mhz
                            ?.value ?? null}
                        bind:enabled={
                            powerConfig[activeProfile].max_freq_mhz.enabled
                        }
                        bind:value={
                            powerConfig[activeProfile].max_freq_mhz.value
                        }
                        on:change={onChangeFreqLimits}
                    />
                </div>
            {/if}
        </div>
    {/if}

    {#if errorMessage}
        <div class="text-xs text-error mt-2">{errorMessage}</div>
    {/if}
</div>
