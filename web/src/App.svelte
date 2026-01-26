<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { DefaultService } from "./api";
    import DeviceHeader from "./components/DeviceHeader.svelte";
    import FanControl from "./components/FanControl.svelte";
    import PowerControl from "./components/PowerControl.svelte";
    import BatteryControl from "./components/BatteryControl.svelte";
    import Sensors from "./components/Sensors.svelte";
    import Panel from "./components/Panel.svelte";
    import { OpenAPI } from "./api";
    import VersionMismatchModal from "./components/VersionMismatchModal.svelte";
    import { gtSemver } from "./lib/semver";
    import Icon from "@iconify/svelte";

    let healthy: boolean = false;
    let cliPresent: boolean = true;

    let pollId: ReturnType<typeof setInterval> | null = null;

    // Hosted vs embedded detection
    const apiOrigin = new URL(OpenAPI.BASE || "/api", window.location.href)
        .origin;
    const isHosted = window.location.origin !== apiOrigin;

    // Service update info for mismatch gate
    let serviceCurrentVersion: string | null = null;
    let serviceLatestVersion: string | null = null;
    let showMismatchGate = false;

    onMount(async () => {
        await pollHealthOnce();
        pollId = setInterval(async () => {
            await pollHealthOnce();
        }, 1000);
        // One-shot update check to decide mismatch gating (ignore paused setting)
        try {
            const res = await DefaultService.checkUpdate();
            serviceCurrentVersion =
                (res.current_version ?? null)?.toString().trim() || null;
            serviceLatestVersion =
                (res.latest_version ?? null)?.toString().trim() || null;
            const updateAvailable =
                serviceCurrentVersion && serviceLatestVersion
                    ? gtSemver(serviceLatestVersion, serviceCurrentVersion)
                    : false;
            showMismatchGate = isHosted && updateAvailable;
        } catch {}
    });

    async function pollHealthOnce() {
        try {
            const res = await DefaultService.health();
            healthy = true;
            cliPresent = res.cli_present;
        } catch {
            healthy = false;
        }
    }
    onDestroy(() => {
        if (pollId) clearInterval(pollId);
    });
</script>

<main class="min-h-screen flex items-center justify-center p-6">
    <div
        class="w-full max-w-6xl mx-auto space-y-4"
        inert={showMismatchGate}
        aria-hidden={showMismatchGate}
    >
        <section>
            <DeviceHeader {healthy} {cliPresent} />
        </section>

        <section
            class={"flex flex-wrap " +
                (healthy ? "items-start" : "items-stretch") +
                " gap-4"}
        >
            {#each ["telemetry", "fan", "power", "battery"] as pid (pid)}
                <div class={"w-full lg:w-[calc(50%-0.51rem)]"}>
                    {#if pid === "telemetry"}
                        <Panel title="Sensors" expandable={healthy}>
                            <svelte:fragment slot="header">
                                {#if !(healthy && cliPresent)}
                                    <span
                                        class="flex items-center gap-1 opacity-70"
                                    >
                                        <Icon
                                            icon="mdi:thermometer"
                                            class="w-4 h-4"
                                        />
                                        <Icon icon="mdi:fan" class="w-4 h-4" />
                                        <Icon
                                            icon="mdi:chart-timeline-variant"
                                            class="w-4 h-4"
                                        />
                                    </span>
                                {/if}
                            </svelte:fragment>
                            {#if healthy && cliPresent}
                                <Sensors />
                            {:else}
                                <div
                                    class="flex-1 flex flex-col justify-evenly px-3 pb-1"
                                >
                                    <div class="text-sm opacity-80 mb-2">
                                        Local sensor data and fan RPM are read
                                        by the service.
                                    </div>
                                    <ul
                                        class="list-disc list-inside text-sm opacity-80 space-y-1"
                                    >
                                        <li>
                                            Temperature sensors for the CPU,
                                            APU, VRAM, dGPU and other components
                                        </li>
                                        <li>
                                            Historical graph with live updates
                                        </li>
                                        <li>
                                            The sensor history window and
                                            polling rate is adjustable
                                        </li>
                                    </ul>
                                </div>
                            {/if}
                        </Panel>
                    {:else if pid === "fan"}
                        <Panel title="Fan Control" expandable={healthy}>
                            <svelte:fragment slot="header">
                                {#if !healthy}
                                    <span
                                        class="flex items-center gap-1 opacity-70"
                                    >
                                        <Icon icon="mdi:fan" class="w-4 h-4" />
                                        <Icon icon="mdi:tune" class="w-4 h-4" />
                                        <Icon
                                            icon="mdi:chart-bell-curve"
                                            class="w-4 h-4"
                                        />
                                    </span>
                                {/if}
                            </svelte:fragment>
                            {#if healthy && cliPresent}
                                <FanControl />
                            {:else}
                                <div
                                    class="flex-1 flex flex-col justify-evenly px-3 pb-1"
                                >
                                    <div class="text-sm opacity-80 mb-2">
                                        Choose auto, a fixed duty, or customize
                                        your own curve.
                                    </div>
                                    <ul
                                        class="list-disc list-inside text-sm opacity-80 space-y-1"
                                    >
                                        <li>
                                            Take control of your fan speed and
                                            noise.
                                        </li>
                                        <li>
                                            Settings persist and apply at boot.
                                        </li>
                                        <li>
                                            Piecewise points with hysteresis and
                                            rate limit
                                        </li>
                                    </ul>
                                </div>
                            {/if}
                        </Panel>
                    {:else if pid === "power"}
                        <Panel title="Power" expandable={healthy}>
                            <svelte:fragment slot="header">
                                {#if !(healthy && cliPresent)}
                                    <span
                                        class="flex items-center gap-1 opacity-70"
                                    >
                                        <Icon
                                            icon="mdi:power-plug-outline"
                                            class="w-4 h-4"
                                        />
                                        <Icon
                                            icon="mdi:cpu-64-bit"
                                            class="w-4 h-4"
                                        />
                                        <Icon
                                            icon="mdi:thermometer"
                                            class="w-4 h-4"
                                        />
                                    </span>
                                {/if}
                            </svelte:fragment>
                            {#if healthy && cliPresent}
                                <PowerControl />
                            {:else}
                                <div
                                    class="flex-1 flex flex-col justify-evenly px-3 pb-1"
                                >
                                    <div class="text-sm opacity-80 mb-2">
                                        Change your TDP and thermal limit, see
                                        the live values. Powered by RyzenAdj.
                                    </div>
                                    <ul
                                        class="list-disc list-inside text-sm opacity-80 space-y-1"
                                    >
                                        <li>
                                            TDP and thermal limit controls that
                                            persist and apply at boot
                                        </li>
                                        <li>
                                            Allow setting different limits for
                                            different AC states
                                        </li>
                                        <li>
                                            See your charger wattage and make
                                            sure you're not overloading your SoC
                                        </li>
                                    </ul>
                                </div>
                            {/if}
                        </Panel>
                    {:else if pid === "battery"}
                        <Panel title="Battery" expandable={healthy}>
                            <svelte:fragment slot="header">
                                {#if !(healthy && cliPresent)}
                                    <span
                                        class="flex items-center gap-1 opacity-70"
                                    >
                                        <Icon
                                            icon="mdi:battery-80"
                                            class="w-4 h-4"
                                        />
                                        <Icon
                                            icon="mdi:battery-charging-80"
                                            class="w-4 h-4"
                                        />
                                        <Icon
                                            icon="mdi:gauge"
                                            class="w-4 h-4"
                                        />
                                    </span>
                                {/if}
                            </svelte:fragment>
                            {#if healthy && cliPresent}
                                <BatteryControl />
                            {:else}
                                <div
                                    class="flex-1 flex flex-col justify-evenly px-3 pb-1"
                                >
                                    <div class="text-sm opacity-80 mb-2">
                                        View battery live stats and change the
                                        maximum charge limit.
                                    </div>
                                    <ul
                                        class="list-disc list-inside text-sm opacity-80 space-y-1"
                                    >
                                        <li>
                                            Live: Battery charge/discharge rate,
                                            battery health, cycles, etc.
                                        </li>
                                        <li>
                                            Charge rate limit: Set the maximum
                                            charge rate
                                        </li>
                                        <li>
                                            State of charge threshold for rate
                                            limit
                                        </li>
                                    </ul>
                                </div>
                            {/if}
                        </Panel>
                    {/if}
                </div>
            {/each}
        </section>
    </div>
    {#if showMismatchGate}
        <VersionMismatchModal
            serviceCurrent={serviceCurrentVersion}
            serviceLatest={serviceLatestVersion}
            {apiOrigin}
        />
    {/if}
</main>
