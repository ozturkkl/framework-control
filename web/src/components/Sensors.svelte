<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import Icon from "@iconify/svelte";
  import { DefaultService, type TelemetryConfig } from "../api";
  import { OpenAPI } from "../api";
  import MultiSelect from "./MultiSelect.svelte";
  import UiSlider from "./UiSlider.svelte";
  import { hashColor } from "../lib/seriesColors";
  import GraphPanel from "./GraphPanel.svelte";
  import { tooltip } from "../lib/tooltip";

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

  // colors provided by seriesColors

  // Inline graph (was TelemetryGraph.svelte)
  let yMin = 0;
  let yMax = 100;

  // Time domain in ms
  let allTimes: number[] = [];
  let tMin: number | null = null;
  let tMax: number | null = null;
  $: allTimes = Object.values(series)
    .flat()
    .map((p) => p[0]);
  $: tMin = allTimes.length ? Math.min(...allTimes) : null;
  $: tMax = allTimes.length ? Math.max(...allTimes) : null;

  // SVG and layout
  const padding = { left: 36, right: 12, top: 12, bottom: 22 };
  let svgWidth = 400;
  let svgHeight = 220;
  let svgEl: SVGSVGElement;

  function xToPx(x: number) {
    if (tMin == null || tMax == null || tMax === tMin) return padding.left;
    const w = svgWidth - padding.left - padding.right;
    return padding.left + ((x - tMin) / (tMax - tMin)) * w;
  }
  function yToPx(y: number) {
    const h = svgHeight - padding.top - padding.bottom;
    return padding.top + (1 - (y - yMin) / (yMax - yMin)) * h;
  }

  function buildPath(points: Array<[number, number]>) {
    if (!points.length) return "";
    return points
      .map((p, i) => `${i === 0 ? "M" : "L"}${xToPx(p[0])},${yToPx(p[1])}`)
      .join(" ");
  }

  // Hover / crosshair state
  type HoverInfo = { ts: number; name: string; value: number } | null;
  let hover: HoverInfo = null;
  let hoverCssX = 0;
  let hoverCssY = 0;
  let hoverCircleEl: SVGCircleElement | null = null;

  function findNearestPoint(
    pts: Array<[number, number]>,
    targetTs: number
  ): [number, number] | null {
    if (!pts.length) return null;
    let lo = 0;
    let hi = pts.length - 1;
    while (lo < hi) {
      const mid = Math.floor((lo + hi) / 2);
      if (pts[mid][0] < targetTs) lo = mid + 1;
      else hi = mid;
    }
    const i2 = lo;
    const i1 = Math.max(0, lo - 1);
    const p1 = pts[i1];
    const p2 = pts[i2] ?? pts[pts.length - 1];
    return Math.abs(p2[0] - targetTs) < Math.abs(p1[0] - targetTs) ? p2 : p1;
  }

  function onMouseMove(e: MouseEvent) {
    if (!svgEl || tMin == null || tMax == null) return;
    const rect = svgEl.getBoundingClientRect();
    const relX = e.clientX - rect.left;
    const relY = e.clientY - rect.top;
    const fracX = Math.max(0, Math.min(1, relX / Math.max(1, rect.width)));
    const xView = fracX * svgWidth;

    const w = svgWidth - padding.left - padding.right;
    if (w <= 0) return;
    const targetTs = tMin + ((xView - padding.left) / w) * (tMax - tMin);

    const scaleX = rect.width / svgWidth;
    const scaleY = rect.height / svgHeight;

    // Choose series by vertical proximity to cursor (pixel space), break ties by time proximity
    let best: {
      ts: number;
      name: string;
      value: number;
      vDist: number;
      tDist: number;
    } | null = null;
    for (const [name, pts] of Object.entries(series)) {
      if (!pts?.length) continue;
      const p = findNearestPoint(pts, targetTs);
      if (!p) continue;
      const yPx = yToPx(p[1]) * scaleY;
      const vDist = Math.abs(yPx - relY);
      const tDist = Math.abs(p[0] - targetTs);
      if (
        !best ||
        vDist < best.vDist ||
        (vDist === best.vDist && tDist < best.tDist)
      ) {
        best = { ts: p[0], name, value: p[1], vDist, tDist };
      }
    }
    if (!best) {
      hover = null;
      return;
    }
    hover = { ts: best.ts, name: best.name, value: best.value };

    // Compute CSS pixel position for tooltip
    hoverCssX = xToPx(best.ts) * scaleX;
    hoverCssY = yToPx(best.value) * scaleY;
  }
  function onMouseLeave() {
    hover = null;
  }

  function formatTick(ms: number) {
    const s = Math.round(ms / 1000);
    const m = Math.floor(s / 60);
    const ss = String(s % 60).padStart(2, "0");
    return `${m}:${ss}`;
  }

  // Compute nice time ticks (~6 ticks)
  $: timeTicks = (() => {
    if (tMin == null || tMax == null) return [] as number[];
    const span = Math.max(1, tMax - tMin);
    const target = 6;
    const stepCandidates = [5, 10, 15, 30, 60, 120, 300].map((s) => s * 1000);
    const approx = span / target;
    let step = stepCandidates[0];
    for (const c of stepCandidates)
      if (c >= approx) {
        step = c;
        break;
      }
    const first = Math.ceil(tMin / step) * step;
    const ticks: number[] = [];
    for (let t = first; t <= tMax; t += step) ticks.push(t);
    return ticks;
  })();
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

<GraphPanel>
  <svelte:fragment slot="top" let:openSettings>
    <!-- Inline legend on the left -->
    <div class="flex flex-wrap items-center gap-2 text-xs gap-y-1 pl-[2px]">
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
        on:click={openSettings}
        aria-label="Open settings"
      >
        <Icon icon="mdi:cog-outline" class="text-base" />
      </button>
    </div>
  </svelte:fragment>

  <svelte:fragment slot="graph">
    <div class="relative">
      <svg
        bind:this={svgEl}
        class="w-full bg-base-100 rounded border border-base-300"
        viewBox={`0 0 ${svgWidth} ${svgHeight}`}
        role="img"
        aria-label="Temperature sensors graph"
        on:mousemove={onMouseMove}
        on:mouseleave={onMouseLeave}
      >
        <!-- axes -->
        <g stroke="currentColor" class="opacity-30">
          <line
            x1={padding.left}
            y1={yToPx(yMin)}
            x2={svgWidth - padding.right}
            y2={yToPx(yMin)}
            stroke-width="1"
          />
          <line
            x1={padding.left}
            y1={padding.top}
            x2={padding.left}
            y2={svgHeight - padding.bottom}
            stroke-width="1"
          />
        </g>

        <!-- horizontal gridlines and labels -->
        {#each [0, 20, 40, 60, 80, 100] as d}
          <g>
            <line
              x1={padding.left}
              y1={yToPx(d)}
              x2={svgWidth - padding.right}
              y2={yToPx(d)}
              stroke="currentColor"
              class="opacity-10"
            />
            <text
              x={padding.left - 6}
              y={yToPx(d) + 4}
              text-anchor="end"
              class="fill-current opacity-60 text-[10px]">{d}°C</text
            >
          </g>
        {/each}

        <!-- vertical time gridlines -->
        {#each timeTicks as t}
          <g>
            <line
              x1={xToPx(t)}
              y1={padding.top}
              x2={xToPx(t)}
              y2={svgHeight - padding.bottom}
              stroke="currentColor"
              class="opacity-10"
            />
            <text
              x={xToPx(t)}
              y={svgHeight - padding.bottom + 16}
              text-anchor="middle"
              class="fill-current opacity-60 text-[10px]"
              >{formatTick(t - (tMin ?? 0))}</text
            >
          </g>
        {/each}

        <!-- series lines -->
        {#each Object.entries(series) as [name, pts]}
          <path
            d={buildPath(pts)}
            fill="none"
            stroke={hashColor(name)}
            stroke-width="2"
          />
        {/each}

        <!-- crosshair + marker -->
        {#if hover}
          <line
            x1={xToPx(hover.ts)}
            y1={padding.top}
            x2={xToPx(hover.ts)}
            y2={svgHeight - padding.bottom}
            stroke="currentColor"
            class="opacity-40"
            stroke-dasharray="3,3"
          />
          <circle
            bind:this={hoverCircleEl}
            cx={xToPx(hover.ts)}
            cy={yToPx(hover.value)}
            r="3.5"
            fill={hashColor(hover.name)}
            stroke="white"
            stroke-width="1.5"
          />
        {/if}
      </svg>

      <!-- Minimal tooltip element rendered once; action portals and positions it -->
      <div
        use:tooltip={{
          anchor: () => hoverCircleEl,
          visible: !!hover,
          attachGlobalDismiss: false,
        }}
        class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs flex items-center gap-2"
      >
        {#if hover}
          <span
            class="inline-block w-2.5 h-2.5 rounded-sm"
            style={`background:${hashColor(hover.name)}`}
          ></span>
          <span class="opacity-80">{hover.name}</span>
          <span class="font-medium">{hover.value.toFixed(1)}°C</span>
        {/if}
      </div>
    </div>
  </svelte:fragment>

  <svelte:fragment slot="bottom">
    <MultiSelect
      items={availableSensors}
      bind:selected={selectedSensors}
      label="Sensors"
      on:change={() => {
        saveSelectedSensors();
        fetchHistory();
      }}
    />
  </svelte:fragment>

  <svelte:fragment slot="settings">
    <div class="flex-1 flex flex-col justify-evenly space-y-2">
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
  </svelte:fragment>
</GraphPanel>
