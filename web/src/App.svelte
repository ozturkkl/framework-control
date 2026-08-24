<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { flip } from "svelte/animate";
    import {
        SHADOW_ITEM_MARKER_PROPERTY_NAME,
        SHADOW_PLACEHOLDER_ITEM_ID,
        dragHandleZone,
        type DndEvent,
    } from "svelte-dnd-action";
    import {
        DefaultService,
        OpenAPI,
        type DashboardPanel,
        type DashboardPanelId,
    } from "./api";
    import DeviceHeader from "./components/DeviceHeader.svelte";
    import FanControl from "./components/FanControl.svelte";
    import PowerControl from "./components/PowerControl.svelte";
    import BatteryControl from "./components/BatteryControl.svelte";
    import Sensors from "./components/Sensors.svelte";
    import Panel from "./components/Panel.svelte";
    import VersionMismatchModal from "./components/VersionMismatchModal.svelte";
    import { gtSemver } from "./lib/semver";
    import { followConfig, patch } from "./lib/config";
    import Icon from "@iconify/svelte";

    let healthy = false;
    let cliPresent = true;

    const flipDurationMs = 100;

    const PANEL_META: Record<
        DashboardPanelId,
        { title: string; icons: string[]; intro: string; points: string[] }
    > = {
        telemetry: {
            title: "Sensors",
            icons: [
                "mdi:thermometer",
                "mdi:fan",
                "mdi:chart-timeline-variant",
            ],
            intro: "Local sensor data and fan RPM are read by the service.",
            points: [
                "Temperature sensors for the CPU, APU, VRAM, dGPU and other components",
                "Historical graph with live updates",
                "The sensor history window and polling rate is adjustable",
            ],
        },
        fan: {
            title: "Fan Control",
            icons: ["mdi:fan", "mdi:tune", "mdi:chart-bell-curve"],
            intro: "Choose auto, a fixed duty, or customize your own curve.",
            points: [
                "Take control of your fan speed and noise.",
                "Settings persist and apply at boot.",
                "Piecewise points with hysteresis and rate limit",
            ],
        },
        power: {
            title: "Power",
            icons: [
                "mdi:power-plug-outline",
                "mdi:cpu-64-bit",
                "mdi:thermometer",
            ],
            intro: "Change your TDP and thermal limit, see the live values. Powered by RyzenAdj.",
            points: [
                "TDP and thermal limit controls that persist and apply at boot",
                "Allow setting different limits for different AC states",
                "See your charger wattage and make sure you're not overloading your SoC",
            ],
        },
        battery: {
            title: "Battery",
            icons: [
                "mdi:battery-80",
                "mdi:battery-charging-80",
                "mdi:gauge",
            ],
            intro: "View battery live stats and change the maximum charge limit.",
            points: [
                "Live: Battery charge/discharge rate, battery health, cycles, etc.",
                "Charge rate limit: Set the maximum charge rate",
                "State of charge threshold for rate limit",
                "History graph of charge level and charge/discharge power",
            ],
        },
    };

    const PANEL_IDS: DashboardPanelId[] = [
        "telemetry",
        "fan",
        "power",
        "battery",
    ];
    let pollId: ReturnType<typeof setInterval> | null = null;

    const apiOrigin = new URL(OpenAPI.BASE || "/api", window.location.href)
        .origin;
    const isHosted = window.location.origin !== apiOrigin;

    let serviceCurrentVersion: string | null = null;
    let serviceLatestVersion: string | null = null;
    let showMismatchGate = false;

    let editing = false;
    let dragActive = false;
    let layoutError: string | null = null;
    let layout: DashboardPanel[] = completeLayout([]);
    let items: DashboardPanel[] = [];

    $: live = healthy && cliPresent;
    $: disabledCount = layout.filter((panel) => !panel.enabled).length;
    $: if (!dragActive) {
        items = (editing ? layout : layout.filter((panel) => panel.enabled)).map(
            clonePanel,
        );
    }
    $: dndOptions = {
        items,
        flipDurationMs,
        dragDisabled: !editing,
        dropFromOthersDisabled: true,
        delayTouchStart: true,
        dropTargetStyle: {},
        zoneTabIndex: editing ? 0 : -1,
        type: "fc-dashboard-panels",
    };

    onMount(async () => {
        await pollHealthOnce();
        pollId = setInterval(async () => {
            await pollHealthOnce();
        }, 1000);
        try {
            const res = await DefaultService.checkUpdate();
            serviceCurrentVersion =
                (res.current_version ?? null)?.toString().trim() || null;
            serviceLatestVersion =
                (res.latest_version ?? null)?.toString().trim() || null;
            showMismatchGate =
                isHosted &&
                !!serviceCurrentVersion &&
                !!serviceLatestVersion &&
                gtSemver(serviceLatestVersion, serviceCurrentVersion);
        } catch {}
    });

    async function pollHealthOnce() {
        try {
            const res = await DefaultService.health();
            healthy = true;
            cliPresent = res.cli_present;
        } catch {
            healthy = false;
            editing = false;
        }
    }

    function clonePanel(panel: DashboardPanel): DashboardPanel {
        return { id: panel.id, enabled: panel.enabled, size: panel.size };
    }

    function completeLayout(panels: DashboardPanel[]): DashboardPanel[] {
        const seen = new Set<DashboardPanelId>();
        const out: DashboardPanel[] = [];
        for (const panel of panels) {
            if (SHADOW_ITEM_MARKER_PROPERTY_NAME in panel) continue;
            if (!(panel.id in PANEL_META) || seen.has(panel.id)) continue;
            seen.add(panel.id);
            out.push(clonePanel(panel));
        }
        for (const id of PANEL_IDS) {
            if (!seen.has(id)) out.push({ id, enabled: true, size: "half" });
        }
        return out;
    }

    function isPlaceholder(panel: DashboardPanel): boolean {
        return String(panel.id) === SHADOW_PLACEHOLDER_ITEM_ID;
    }

    function panelTitle(panel: DashboardPanel): string {
        return PANEL_META[panel.id]?.title ?? String(panel.id);
    }

    function gridSpan(panel: DashboardPanel): string {
        return panel.size === "full"
            ? "col-span-12"
            : "col-span-12 lg:col-span-6";
    }

    function commitLayout(next: DashboardPanel[]) {
        layout = completeLayout(next);
        layoutError = null;
        patch({ ui: { panels: layout } }).catch((e) => {
            layoutError = e instanceof Error ? e.message : String(e);
        });
    }

    function applyRemotePanels(panels: DashboardPanel[] | null) {
        if (dragActive) return;
        layout = completeLayout(panels ?? []);
    }

    const stopFollow = followConfig({
        select: (c) => c.ui?.panels ?? null,
        apply: applyRemotePanels,
    });

    onDestroy(() => {
        if (pollId) clearInterval(pollId);
        stopFollow();
    });

    function zoneItemKey(panel: DashboardPanel): string {
        const id = String(panel.id);
        if (isPlaceholder(panel)) return id;
        if (SHADOW_ITEM_MARKER_PROPERTY_NAME in panel) return `${id}-shadow`;
        return id;
    }

    function handleConsider(event: CustomEvent<DndEvent<DashboardPanel>>) {
        dragActive = true;
        items = event.detail.items;
    }

    function handleFinalize(event: CustomEvent<DndEvent<DashboardPanel>>) {
        dragActive = false;
        if (editing) commitLayout(event.detail.items);
    }

    function mapPanel(
        id: DashboardPanelId,
        fn: (panel: DashboardPanel) => DashboardPanel,
    ) {
        commitLayout(layout.map((panel) => (panel.id === id ? fn(panel) : panel)));
    }

    function setPanelEnabled(id: DashboardPanelId, enabled: boolean) {
        mapPanel(id, (panel) => ({ ...panel, enabled }));
    }

    function toggleSize(id: DashboardPanelId) {
        mapPanel(id, (panel) => ({
            ...panel,
            size: panel.size === "full" ? "half" : "full",
        }));
    }

    function neighbor(
        id: DashboardPanelId,
        delta: -1 | 1,
    ): readonly [number, number] | null {
        const index = layout.findIndex((panel) => panel.id === id);
        const other = index + delta;
        if (index < 0 || other < 0 || other >= layout.length) return null;
        return [index, other];
    }

    function movePanel(id: DashboardPanelId, delta: -1 | 1) {
        const pair = neighbor(id, delta);
        if (!pair) return;
        const [index, other] = pair;
        const next = [...layout];
        [next[index], next[other]] = [next[other], next[index]];
        commitLayout(next);
    }

    function canMove(id: DashboardPanelId, delta: -1 | 1): boolean {
        return neighbor(id, delta) != null;
    }
</script>

<main class="min-h-screen flex items-center justify-center p-6">
    <div
        class="w-full max-w-6xl mx-auto space-y-4"
        inert={showMismatchGate}
        aria-hidden={showMismatchGate}
    >
        <section>
            <DeviceHeader
                {healthy}
                {cliPresent}
                {editing}
                on:toggleEdit={() => (editing = !editing)}
            />
        </section>

        {#if editing}
            <div
                class="flex flex-wrap items-center gap-2 px-1 text-sm"
                role="toolbar"
                aria-label="Edit dashboard layout"
            >
                <span class="opacity-70">
                    Editing layout{#if disabledCount > 0}
                        · {disabledCount} disabled{/if}
                </span>
                {#if layoutError}
                    <span class="text-error text-xs">{layoutError}</span>
                {/if}
                <div class="flex items-center gap-2 ml-auto">
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        title="Restore the default panel order, enabled state, and sizes"
                        on:click={() => commitLayout([])}
                    >
                        Reset layout
                    </button>
                    <button
                        type="button"
                        class="btn btn-primary btn-xs"
                        on:click={() => (editing = false)}
                    >
                        Done
                    </button>
                </div>
            </div>
        {/if}

        {#if items.length === 0 && !editing}
            <p class="text-sm opacity-70" role="status">
                All panels are disabled. Use the layout button in the header to
                enable them.
            </p>
        {/if}

        <section
            class="grid grid-cols-12 gap-4 items-stretch"
            aria-label="Dashboard panels"
            use:dragHandleZone={dndOptions}
            on:consider={handleConsider}
            on:finalize={handleFinalize}
        >
            {#each items as panel (zoneItemKey(panel))}
                <div
                    class="h-full self-stretch {gridSpan(panel)}"
                    role="group"
                    aria-label={panelTitle(panel)}
                    animate:flip={{ duration: flipDurationMs }}
                >
                    {#if !isPlaceholder(panel)}
                        <Panel
                            title={panelTitle(panel)}
                            {editing}
                            enabled={panel.enabled}
                            size={panel.size}
                            canMovePrev={canMove(panel.id, -1)}
                            canMoveNext={canMove(panel.id, 1)}
                            on:disable={() => setPanelEnabled(panel.id, false)}
                            on:enable={() => setPanelEnabled(panel.id, true)}
                            on:toggleSize={() => toggleSize(panel.id)}
                            on:movePrev={() => movePanel(panel.id, -1)}
                            on:moveNext={() => movePanel(panel.id, 1)}
                        >
                            <svelte:fragment slot="header">
                                {#if !live}
                                    <span
                                        class="flex items-center gap-1 opacity-70"
                                    >
                                        {#each PANEL_META[panel.id].icons as icon (icon)}
                                            <Icon {icon} class="w-4 h-4" />
                                        {/each}
                                    </span>
                                {/if}
                            </svelte:fragment>
                            {#if live}
                                {#if panel.id === "telemetry"}
                                    <Sensors />
                                {:else if panel.id === "fan"}
                                    <FanControl />
                                {:else if panel.id === "power"}
                                    <PowerControl />
                                {:else if panel.id === "battery"}
                                    <BatteryControl />
                                {/if}
                            {:else}
                                {@const meta = PANEL_META[panel.id]}
                                <div
                                    class="flex-1 flex flex-col justify-evenly px-3 pb-1"
                                >
                                    <div class="text-sm opacity-80 mb-2">
                                        {meta.intro}
                                    </div>
                                    <ul
                                        class="list-disc list-inside text-sm opacity-80 space-y-1"
                                    >
                                        {#each meta.points as point (point)}
                                            <li>{point}</li>
                                        {/each}
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
