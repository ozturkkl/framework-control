<script lang="ts">
  import { onMount } from "svelte";
  import { DefaultService } from "../api";
  import type { Config, FanMode, FanCurveConfig, PartialConfig } from "../api";
  import { throttleDebounce } from "../lib/utils";
  import Icon from "@iconify/svelte";

  let error: string | null = null;
  let success: string | null = null;
  let token = import.meta.env.VITE_CONTROL_TOKEN;
  let successTimeout: ReturnType<typeof setTimeout> | null = null;

  // Centralized defaults for the fan curve
  const DEFAULT_FAN_CURVE: FanCurveConfig = {
    enabled: false,
    mode: "Auto",
    sensor: "APU",
    points: [
      [50, 0],
      [75, 30],
      [90, 50],
    ],
    poll_ms: 1500,
    hysteresis_c: 2,
    rate_limit_pct_per_step: 100,
    manual_duty_pct: 50,
  };

  export let mode: FanMode = DEFAULT_FAN_CURVE.mode;
  let suppressModeSave = true;
  let prevMode: FanMode = mode;
  let manualDutyPct = DEFAULT_FAN_CURVE.manual_duty_pct;
  let sensor: string = DEFAULT_FAN_CURVE.sensor;
  let currentFanCurve: FanCurveConfig | null = null;

  // Curve editor state
  type Point = [number, number];
  let points: Point[] = DEFAULT_FAN_CURVE.points as Point[];
  let pollMs = DEFAULT_FAN_CURVE.poll_ms;
  let hysteresisC = DEFAULT_FAN_CURVE.hysteresis_c;
  let rateLimitPctPerStep = DEFAULT_FAN_CURVE.rate_limit_pct_per_step;

  // Graph dimensions
  const minTemp = 0;
  const maxTemp = 100;
  const minDuty = 0;
  const maxDuty = 100;
  const padding = { left: 36, right: 18, top: 12, bottom: 28 };
  const editableMinTemp = 1;
  let svgEl: SVGSVGElement;
  let svgWidth = 400;
  let svgHeight = 220;
  let selectedIdx: number | null = null;
  let isDragging = false;
  let dragMoved = false;
  let lastDragged: Point | null = null;
  let dragOffset: { dx: number; dy: number } | null = null;
  let showSettings = false;

  function sortPointsInPlace() {
    points.sort((a, b) => a[0] - b[0]);
    points = points.slice();
  }

  function clamp(n: number, min: number, max: number) {
    return Math.max(min, Math.min(max, n));
  }

  function xToPx(x: number) {
    const w = svgWidth - padding.left - padding.right;
    return padding.left + ((x - minTemp) / (maxTemp - minTemp)) * w;
  }

  function yToPx(y: number) {
    const h = svgHeight - padding.top - padding.bottom;
    return padding.top + (1 - (y - minDuty) / (maxDuty - minDuty)) * h;
  }

  function pxToX(px: number) {
    const w = svgWidth - padding.left - padding.right;
    const t = clamp((px - padding.left) / w, 0, 1);
    return minTemp + t * (maxTemp - minTemp);
  }

  function pxToY(py: number) {
    const h = svgHeight - padding.top - padding.bottom;
    const t = clamp((py - padding.top) / h, 0, 1);
    return minDuty + (1 - t) * (maxDuty - minDuty);
  }

  function buildPath(pts: Point[]) {
    if (!pts.length) return "";
    const segs = pts.map(
      (p, i) => `${i === 0 ? "M" : "L"}${xToPx(p[0])},${yToPx(p[1])}`
    );
    return segs.join(" ");
  }

  function buildArea(pts: Point[]) {
    if (pts.length < 2) return "";
    const baseY = yToPx(0);
    const startX = xToPx(pts[0][0]);
    const endX = xToPx(pts[pts.length - 1][0]);
    const line = buildPath(pts);
    return `${line} L${endX},${baseY} L${startX},${baseY} Z`;
  }

  $: sortedPoints = [...points].sort((a, b) => a[0] - b[0]);
  $: sortedWithAnchors = ([[0, 0]] as Point[])
    .concat(sortedPoints)
    .concat([[100, 100]] as Point[]);
  $: pathLine = buildPath(sortedWithAnchors);
  $: pathArea = buildArea(sortedWithAnchors);

  onMount(async () => {
    try {
      const { ok, config } = await DefaultService.getConfig();
      if (ok && config?.fan_curve) {
        currentFanCurve = config.fan_curve;
        mode = currentFanCurve.mode ?? mode;
        sensor = currentFanCurve.sensor ?? sensor;
        if (Array.isArray(currentFanCurve.points)) {
          points = currentFanCurve.points.map((p) => [p[0], p[1]] as Point);
          sortPointsInPlace();
        }
        if (typeof currentFanCurve.poll_ms === "number")
          pollMs = currentFanCurve.poll_ms;
        if (typeof currentFanCurve.hysteresis_c === "number")
          hysteresisC = currentFanCurve.hysteresis_c;
        if (typeof currentFanCurve.rate_limit_pct_per_step === "number")
          rateLimitPctPerStep = Math.max(
            1,
            currentFanCurve.rate_limit_pct_per_step
          );
        if (typeof currentFanCurve.manual_duty_pct === "number") {
          manualDutyPct = currentFanCurve.manual_duty_pct;
        }
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    }
    // Sync prevMode to whatever we loaded so lifting suppression won't trigger a save
    prevMode = mode;
    // Allow reactive saves after initial load completes
    suppressModeSave = false;
  });

  const save = throttleDebounce(
    async () => {
      error = null;
      success = null;
      // Merge changes into the current fan curve so we send a full value (backend replaces fully)
      const base: FanCurveConfig = currentFanCurve ?? { ...DEFAULT_FAN_CURVE };
      const updated: FanCurveConfig = {
        ...base,
        enabled: mode !== "Auto",
        mode,
        sensor,
        points: points.map((p) => [Math.round(p[0]), Math.round(p[1])]),
        poll_ms: Math.max(200, Math.floor(pollMs)),
        hysteresis_c: Math.max(0, Math.floor(hysteresisC)),
        rate_limit_pct_per_step: Math.max(1, Math.floor(rateLimitPctPerStep)),
        manual_duty_pct: mode === "Manual" ? manualDutyPct : undefined,
      };
      const patch: PartialConfig = { fan_curve: updated };
      try {
        const res = await DefaultService.setConfig(token, patch);
        if (!res.ok) throw new Error("Failed to save config");
        currentFanCurve = updated;
        if (successTimeout) {
          clearTimeout(successTimeout);
          successTimeout = null;
        }
        success = "Saved";
        successTimeout = setTimeout(() => {
          success = null;
          successTimeout = null;
        }, 750);
      } catch (e: unknown) {
        error = e instanceof Error ? e.message : String(e);
      }
    },
    200,
    false,
    true
  );

  // Apply mode changes coming from parent binding
  $: if (!suppressModeSave && mode !== prevMode) {
    prevMode = mode;
    save();
  }

  // While suppressed, keep prevMode in sync without saving
  $: if (suppressModeSave) {
    prevMode = mode;
  }

  function startDrag(idx: number, ev: PointerEvent) {
    selectedIdx = idx;
    isDragging = true;
    dragMoved = false;
    (ev.target as Element).setPointerCapture?.(ev.pointerId);
    const rect = svgEl.getBoundingClientRect();
    const scaleX = svgWidth / rect.width;
    const scaleY = svgHeight / rect.height;
    const px = (ev.clientX - rect.left) * scaleX;
    const py = (ev.clientY - rect.top) * scaleY;
    const cx = xToPx(points[idx][0]);
    const cy = yToPx(points[idx][1]);
    dragOffset = { dx: px - cx, dy: py - cy };
  }

  function onSvgPointerMove(ev: PointerEvent) {
    if (!isDragging || selectedIdx === null) return;
    dragMoved = true;
    const rect = svgEl.getBoundingClientRect();
    const scaleX = svgWidth / rect.width;
    const scaleY = svgHeight / rect.height;
    let px = (ev.clientX - rect.left) * scaleX;
    let py = (ev.clientY - rect.top) * scaleY;
    if (dragOffset) {
      px -= dragOffset.dx;
      py -= dragOffset.dy;
    }
    const idx = selectedIdx;
    const nx = clamp(pxToX(px), editableMinTemp, maxTemp);
    const ny = clamp(pxToY(py), minDuty, maxDuty);
    points[idx] = [Math.round(nx), Math.round(ny)];
    points = points.slice();
    lastDragged = points[idx];
    save();
  }

  function endDrag(ev: PointerEvent) {
    if (!isDragging) return;
    isDragging = false;
    if (!dragMoved) {
      // simple click selects point without moving
    }
    // Sort and keep selection on the moved point
    sortPointsInPlace();
    points = points.slice();
    if (lastDragged) {
      const found = points.findIndex(
        (p) => p[0] === lastDragged![0] && p[1] === lastDragged![1]
      );
      if (found !== -1) selectedIdx = found;
    }
    lastDragged = null;
    dragOffset = null;
  }

  function addPointAt(ev: MouseEvent) {
    // Use double click to avoid conflict with drags
    const rect = svgEl.getBoundingClientRect();
    const scaleX = svgWidth / rect.width;
    const scaleY = svgHeight / rect.height;
    const px = (ev.clientX - rect.left) * scaleX;
    const py = (ev.clientY - rect.top) * scaleY;
    const nx = clamp(Math.round(pxToX(px)), editableMinTemp, maxTemp);
    const ny = clamp(Math.round(pxToY(py)), minDuty, maxDuty);
    // Insert keeping order; avoid duplicates at same x by nudging
    let insertIdx = points.findIndex((p) => p[0] >= nx);
    if (insertIdx === -1) insertIdx = points.length;
    if (insertIdx > 0 && points[insertIdx - 1][0] === nx)
      points[insertIdx - 1][0] = nx - 1;
    if (insertIdx < points.length && points[insertIdx]?.[0] === nx)
      points[insertIdx][0] = nx + 1;
    points.splice(insertIdx, 0, [nx, ny]);
    points = points.slice();
    sortPointsInPlace();
    save();
  }

  function deletePointAt(index: number) {
    points.splice(index, 1);
    points = points.slice();
    if (selectedIdx === index) selectedIdx = null;
    save();
  }

  function resetToDefaults() {
    points = DEFAULT_FAN_CURVE.points as Point[];
    pollMs = DEFAULT_FAN_CURVE.poll_ms;
    hysteresisC = DEFAULT_FAN_CURVE.hysteresis_c;
    rateLimitPctPerStep = DEFAULT_FAN_CURVE.rate_limit_pct_per_step;
    sortPointsInPlace();
    save();
  }

  function updatePointAt(
    index: number,
    newTemp: number | null,
    newDuty: number | null,
    doSort = false
  ) {
    const x =
      newTemp !== null
        ? clamp(Math.round(newTemp), editableMinTemp, maxTemp)
        : points[index][0];
    const y =
      newDuty !== null
        ? clamp(Math.round(newDuty), minDuty, maxDuty)
        : points[index][1];
    points[index] = [x, y];
    if (doSort) sortPointsInPlace();
    else points = points.slice();
    save();
  }
</script>

<svelte:window on:pointerup={endDrag} on:pointercancel={endDrag} />

<div class="relative min-h-16 flex flex-col justify-center">
  {#if error}
    <div class="alert alert-error text-sm">
      <span>{error}</span>
    </div>
  {/if}

  <span
    class="pointer-events-none select-none absolute top-[-38px] left-[275px] inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-500 text-white shadow transition duration-200 ease-out"
    style="opacity: {success ? 1 : 0}; transform: scale({success ? 1 : 0.9});"
    aria-hidden="true"
  >
    <Icon icon="mdi:check" class="text-base" />
  </span>

  {#if mode === "Auto"}
    <div class="text-lg opacity-80">
      Fan will be controlled by the default firmware curve.
    </div>
  {/if}

  {#if mode === "Manual"}
    <div class="form-control">
      <label class="label" for="manual-duty">
        <span class="label-text">Manual duty: {manualDutyPct}%</span>
      </label>
      <input
        id="manual-duty"
        type="range"
        min="0"
        max="100"
        step="1"
        class="range"
        bind:value={manualDutyPct}
        on:input={save}
      />
    </div>
  {/if}

  {#if mode === "Curve"}
    <div class="grid grid-cols-1 gap-4">
      <div>
        <div class="card bg-base-200 p-3">
          <div class="flex items-center justify-between mb-2">
            {#if showSettings}
              <button
                class="btn btn-xs btn-ghost gap-1"
                on:click={() => (showSettings = false)}
                aria-label="Back to graph"
              >
                <Icon icon="mdi:arrow-left" class="text-base" />
                Back
              </button>
              <div class="flex gap-2">
                <button
                  class="btn btn-xs gap-1"
                  on:click={resetToDefaults}
                  aria-label="Reset"
                >
                  <Icon icon="mdi:backup-restore" class="text-base" />
                  Reset
                </button>
              </div>
            {:else}
              <div class="font-medium">Fan curve</div>
              <div class="flex gap-2">
                <button
                  class="btn btn-xs btn-ghost"
                  on:click={() => (showSettings = true)}
                  aria-label="Open settings"
                >
                  <Icon icon="mdi:cog-outline" class="text-base" />
                </button>
                <button
                  class="btn btn-xs gap-1"
                  on:click={resetToDefaults}
                  aria-label="Reset"
                >
                  <Icon icon="mdi:backup-restore" class="text-base" />
                  Reset
                </button>
              </div>
            {/if}
          </div>
          {#if !showSettings}
            <div class="w-full relative">
              <svg
                bind:this={svgEl}
                class="w-full h-full touch-none select-none bg-base-100 rounded border border-base-300"
                viewBox={`0 0 ${svgWidth} ${svgHeight}`}
                on:dblclick|preventDefault={addPointAt}
                on:pointermove={onSvgPointerMove}
                on:pointerup={endDrag}
                role="application"
                aria-label="Fan curve editor"
              >
                <!-- axes -->
                <g stroke="currentColor" class="opacity-30">
                  <line
                    x1={padding.left}
                    y1={yToPx(0)}
                    x2={svgWidth - padding.right}
                    y2={yToPx(0)}
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

                <!-- gridlines and labels -->
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
                      class="fill-current opacity-60 text-[10px]">{d}%</text
                    >
                  </g>
                {/each}
                {#each [20, 40, 60, 80, 100] as t}
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
                      class="fill-current opacity-60 text-[10px]">{t}°C</text
                    >
                  </g>
                {/each}

                <!-- filled area under curve -->
                <path
                  d={pathArea}
                  fill="oklch(var(--p))"
                  opacity="0.15"
                  stroke="none"
                />

                <!-- curve line -->
                <path
                  d={pathLine}
                  fill="none"
                  stroke="oklch(var(--p))"
                  stroke-width="2.25"
                />

                <!-- points -->
                {#each points as p, i (i)}
                  <g
                    on:pointerdown={(e) => startDrag(i, e)}
                    on:contextmenu|preventDefault={() => deletePointAt(i)}
                    class="cursor-pointer focus:outline-none focus-visible:outline-none"
                    role="button"
                    tabindex="0"
                    aria-label={`Point at ${p[0]}°C ${p[1]}%`}
                  >
                    <circle
                      cx={xToPx(p[0])}
                      cy={yToPx(p[1])}
                      r={selectedIdx === i ? 6.5 : 5.5}
                      fill={isDragging && selectedIdx === i
                        ? "oklch(var(--p))"
                        : selectedIdx === i
                          ? "oklch(var(--p))"
                          : "#ffffff"}
                      stroke={isDragging && selectedIdx === i
                        ? "oklch(var(--pc))"
                        : "oklch(var(--p))"}
                      stroke-width={selectedIdx === i ? 2.25 : 1.5}
                    />
                  </g>
                {/each}
              </svg>
            </div>
            <div
              class="mt-2 flex items-center justify-between gap-2 text-xs opacity-70"
            >
              <div class="flex-1 min-w-0">
                Double‑click to add. Drag to adjust. Right‑click to delete.
              </div>
              <div class="ml-auto shrink-0 whitespace-nowrap">
                <select
                  aria-label="Sensor"
                  class="select select-bordered select-xs"
                  bind:value={sensor}
                  on:change={save}
                >
                  <option value="APU">APU</option>
                  <option value="CPU">CPU</option>
                </select>
              </div>
            </div>
          {:else}
            <div class="p-6 space-y-10">
              <div class="form-control">
                <div class="flex flex-col gap-2">
                  <label for="poll-ms" class="label-text"
                    >Poll interval: {pollMs} ms</label
                  >
                  <input
                    id="poll-ms"
                    type="range"
                    min="200"
                    max="5000"
                    step="100"
                    class="range range-xs w-full"
                    bind:value={pollMs}
                    on:input={save}
                  />
                </div>
              </div>

              <div class="form-control">
                <div class="flex flex-col gap-2">
                  <label for="hysteresis-c" class="label-text"
                    >Hysteresis: {hysteresisC} °C</label
                  >
                  <input
                    id="hysteresis-c"
                    type="range"
                    min="0"
                    max="10"
                    step="1"
                    class="range range-xs w-full"
                    bind:value={hysteresisC}
                    on:input={save}
                  />
                </div>
              </div>

              <div class="form-control">
                <div class="flex flex-col gap-2">
                  <label for="rate-limit" class="label-text"
                    >Rate limit per step: {rateLimitPctPerStep}%</label
                  >
                  <input
                    id="rate-limit"
                    type="range"
                    min="1"
                    max="100"
                    step="5"
                    class="range range-xs w-full"
                    bind:value={rateLimitPctPerStep}
                    on:input={save}
                  />
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
</style>
