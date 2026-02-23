<script lang="ts">
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
    import UiControlCard from "./UiControlCard.svelte";
    import { tooltip } from "../lib/tooltip";
    import { isWindows as getIsWindows } from "../lib/platform";

    const POWER_INFO_CONTAINER_CLASS =
        "flex flex-col h-44 my-0.5 px-6 justify-center gap-2";
    const isWindows = getIsWindows();

    let activeProfile: keyof PowerConfig = "ac";
    const ACTIVE_PROFILE_KEY = "fc.power.activeProfile";
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

    // Power config
    let powerConfig: PowerConfig = {
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

    $: hasAnyPowerCapability =
        !!capabilities &&
        (capabilities.supports_tdp ||
            capabilities.supports_thermal ||
            capabilities.supports_epp ||
            capabilities.supports_governor ||
            capabilities.supports_frequency_limits);

    $: showControls = hasCheckedStatus && hasAnyPowerCapability;

    function recomputeHighTdpUnlocked() {
        if (!capabilities?.supports_tdp) return;
        const acVal = powerConfig.ac?.tdp_watts?.value ?? 0;
        const batVal = powerConfig.battery?.tdp_watts?.value ?? 0;
        highTdpUnlocked = acVal > 120 || batVal > 60;
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

    async function installRyzenAdj() {
        if (!isWindows || !agreed) return;
        installingRyzenAdj = true;
        errorMessage = null;
        try {
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            await DefaultService.installRyzenadj(auth);
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
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            await DefaultService.uninstallRyzenadj(auth);
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
            const auth = `Bearer ${OpenAPI.TOKEN}`;
            const patch: PartialConfig = {
                power: {
                    [profile]: {
                        min_freq_mhz: { enabled: minEnabled, value: minVal },
                        max_freq_mhz: { enabled: maxEnabled, value: maxVal },
                    },
                },
            };
            await DefaultService.setConfig(auth, patch);
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
                {#if currentState.tdp_limit_watts != null}
                    <span class="opacity-60">•</span>
                    <span
                        class="inline-flex items-center gap-1 whitespace-nowrap"
                    >
                        <Icon
                            icon="mdi:flash-outline"
                            class={`w-4 h-4 ${Number(currentState.tdp_limit_watts) > 95 ? "brightness-200" : Number(currentState.tdp_limit_watts) > 60 ? "brightness-150" : "brightness-100"} text-success`}
                        />
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
                            class={`w-4 h-4 ${Number(currentState.thermal_limit_c) > 95 ? "text-error" : Number(currentState.thermal_limit_c) > 90 ? "text-warning" : "text-success"}`}
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
                    class="transition-transform duration-100"
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
                    class="transition-transform duration-100"
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
                    class="transition-transform duration-100"
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
                    class="transition-transform duration-100"
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
                    class="transition-transform duration-100"
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
