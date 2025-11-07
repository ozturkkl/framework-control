<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Icon from "@iconify/svelte";
  import { DefaultService, type TelemetryConfig } from "../api";
  import { OpenAPI } from "../api";
  import MultiSelect from "./MultiSelect.svelte";
  import UiSlider from "./UiSlider.svelte";
  import TelemetryGraph from "./TelemetryGraph.svelte";
  import { hashColor } from "../lib/seriesColors";

  // No power bar here anymore; moved to PowerControl

  // Telemetry graph state
  const SENSOR_SEL_KEY = "fc.telemetry.sensors";
  const WINDOW_KEY = "fc.telemetry.windowSecs";
  const RETAIN_MAX_SECONDS = 1800;
  let availableSensors: string[] = [];
  let selectedSensors: string[] = [];
  let telemetryPollMs: number = 1000;
  let windowSeconds: number = 300; // UI window (persisted)
  let historyTimer: ReturnType<typeof setInterval> | null = null;
  let series: Record<string, Array<[number, number]>> = {};
  let showSettings = false;

  // colors provided by seriesColors

  async function loadTelemetryConfig() {
    try {
      const cfg = await DefaultService.getConfig();
      const tel = cfg.telemetry;
      telemetryPollMs = Number(tel.poll_ms ?? 1000);
      // Restore saved window or default
      try {
        const saved = localStorage.getItem(WINDOW_KEY);
        const parsed = saved ? parseInt(saved, 10) : NaN;
        if (!Number.isNaN(parsed)) {
          windowSeconds = Math.min(Math.max(30, parsed), RETAIN_MAX_SECONDS);
        } else {
          windowSeconds = 300;
        }
      } catch {
        windowSeconds = 300;
      }
    } catch {}
  }

  async function fetchSensors() {
    try {
      const t = await DefaultService.getThermal();
      availableSensors = Object.keys(t.temps);
      if (selectedSensors.length === 0 && availableSensors.length > 0) {
        // Load saved selection or default to all
        try {
          const saved = localStorage.getItem(SENSOR_SEL_KEY);
          if (saved) selectedSensors = JSON.parse(saved);
        } catch {}
        if (selectedSensors.length === 0)
          selectedSensors = availableSensors.slice();
      }
    } catch {}
  }

  async function fetchHistory() {
    try {
      const data = await DefaultService.getThermalHistory();
      let samples: Array<{ ts_ms: number; temps: Record<string, number> }> =
        data || [];
      // Client-side window filtering
      const cutoff = Date.now() - windowSeconds * 1000;
      samples = samples.filter((s) => s.ts_ms >= cutoff);
      // Build per-sensor series
      const ser: Record<string, Array<[number, number]>> = {};
      for (const s of samples) {
        for (const [name, temp] of Object.entries(s.temps || {})) {
          if (selectedSensors.length && !selectedSensors.includes(name))
            continue;
          if (!ser[name]) ser[name] = [];
          ser[name].push([s.ts_ms, Number(temp)]);
        }
      }
      series = ser;
    } catch {}
  }

  function saveSelectedSensors() {
    try {
      localStorage.setItem(SENSOR_SEL_KEY, JSON.stringify(selectedSensors));
    } catch {}
  }

  function saveWindow() {
    try {
      localStorage.setItem(WINDOW_KEY, String(windowSeconds));
    } catch {}
  }

  async function saveTelemetryConfig() {
    try {
      const auth = `Bearer ${OpenAPI.TOKEN}`;
      const patch: TelemetryConfig = {
        poll_ms: telemetryPollMs,
        // For now, hardcode the retain seconds to 1800 to test.
        retain_seconds: 1800,
      };
      await DefaultService.setConfig(auth, {
        telemetry: patch,
      });

      // Tie history refresh to the configured interval
      const interval = Math.max(200, Math.floor(telemetryPollMs || 1000));
      if (historyTimer) clearInterval(historyTimer);
      historyTimer = setInterval(fetchHistory, interval);
    } catch {}
  }

  onMount(async () => {
    await loadTelemetryConfig();
    await fetchSensors();
    await fetchHistory();
    // Initialize history refresh based on configured polling interval
    const interval = Math.max(200, Math.floor(telemetryPollMs || 1000));
    historyTimer = setInterval(fetchHistory, interval);
  });
  onDestroy(() => {
    if (historyTimer) clearInterval(historyTimer);
  });
</script>

<div class="card bg-base-200 mt-1">
  <div class="px-3 py-1 flex items-center justify-between">
    {#if showSettings}
      <button
        class="btn btn-xs btn-ghost gap-1"
        on:click={() => (showSettings = false)}
        aria-label="Back to chart"
      >
        <Icon icon="mdi:arrow-left" class="text-base" />
        Back
      </button>
      <div></div>
    {:else}
      <!-- Inline legend on the left -->
      <div class="flex flex-wrap items-center gap-2 text-xs gap-y-1">
        {#each selectedSensors as name}
          <span class="inline-flex items-center gap-1">
            <span
              class="w-2.5 h-2.5 rounded-sm"
              style={`background:${hashColor(name)}`}
            ></span>
            <span class="opacity-80">{name}</span>
          </span>
        {/each}
      </div>
      <div class="flex gap-2">
        <button
          class="btn btn-xs btn-ghost"
          on:click={() => (showSettings = true)}
          aria-label="Open settings"
        >
          <Icon icon="mdi:cog-outline" class="text-base" />
        </button>
      </div>
    {/if}
  </div>

  {#if !showSettings}
    <TelemetryGraph {series}>
      <div class="px-2 pb-2 flex justify-end">
        <MultiSelect
          items={availableSensors}
          bind:selected={selectedSensors}
          label="Sensors"
          on:change={() => {
            saveSelectedSensors();
            fetchHistory();
          }}
        />
      </div>
    </TelemetryGraph>
  {:else}
    <div class="space-y-4 pt-2 px-2 pb-3">
      <UiSlider
        label="Poll interval"
        icon="mdi:timer-outline"
        unit="ms"
        min={200}
        max={5000}
        step={100}
        bind:value={telemetryPollMs}
        on:change={saveTelemetryConfig}
      />
      <UiSlider
        label="Window"
        icon="mdi:timeline-clock-outline"
        unit="s"
        min={30}
        max={RETAIN_MAX_SECONDS}
        step={30}
        bind:value={windowSeconds}
        on:change={() => {
          saveWindow();
          fetchHistory();
        }}
      />
    </div>
  {/if}
</div>
