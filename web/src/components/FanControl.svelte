<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { DefaultService } from "../api";
  import type { Config, PartialConfig, PartialFanControlConfig } from "../api";
  import { throttleDebounce } from "../lib/utils";
  import { parseThermalOutput, pickTempForSensor } from "../lib/thermal";
  import { cubicSplineInterpolate } from "../lib/spline";
  import CalibrationModal from "./CalibrationModal.svelte";
  import Icon from "@iconify/svelte";

  let error: string | null = null;
  let success: string | null = null;
  let token = import.meta.env.VITE_CONTROL_TOKEN;
  let successTimeout: ReturnType<typeof setTimeout> | null = null;

  // Live telemetry polling for current temperature and fan RPM
  const LIVE_POLL_MS = 1000;
  let liveTemp: number | null = null;
  let liveRpm: number | null = null;
  let calibrationPoints: [number, number][] | null = null;

  // Centralized defaults for the fan control config (backend schema)
  const DEFAULTS = {
    mode: "disabled",
    curve: {
      sensor: "APU",
      points: [
        [1, 30],
        [70, 30],
        [90, 50],
        [100, 80],
      ] as Point[],
      poll_ms: 400,
      hysteresis_c: 1,
      rate_limit_pct_per_step: 1,
    },
    manual: { duty_pct: 50 },
  };

  export let mode: "Auto" | "Manual" | "Curve" = "Auto";
  let onMountComplete = false;
  let prevMode: typeof mode = mode;
  let manualDutyPct = DEFAULTS.manual.duty_pct;

  // Curve editor state
  type Point = [number, number];
  let points: Point[] = DEFAULTS.curve.points;
  let pollMs = DEFAULTS.curve.poll_ms;
  let hysteresisC = DEFAULTS.curve.hysteresis_c;
  let rateLimitPctPerStep = DEFAULTS.curve.rate_limit_pct_per_step;
  let sensor: string = DEFAULTS.curve.sensor;

  const SHOW_LIVE_KEY = "framework:showLiveRpm";
  let showLive = (() => {
    try {
      const stored = localStorage.getItem(SHOW_LIVE_KEY);
      return stored === "1";
    } catch (_) {
      return false;
    }
  })();
  $: (function persistShowLivePreferenceAndCalibrate() {
    if (showLive) {
      // If enabling and we don't have calibration, start it
      if (onMountComplete && !calibrationPoints) {
        openCalibration();
      }
    }
    try {
      localStorage.setItem(SHOW_LIVE_KEY, showLive ? "1" : "0");
    } catch (_) {}
  })();

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

  // --- Live telemetry helpers moved to lib/thermal ---

  // --- Spline helpers moved to lib/spline ---

  function rpmToPercent(rpm: number): number {
    // Use calibration points if available
    if (calibrationPoints) {
      // Invert the calibration: we have [duty%, rpm] but need rpm -> duty%
      // Create inverted points [rpm, duty%]
      const invertedPoints: [number, number][] = calibrationPoints.map(
        ([duty, rpmVal]) => [rpmVal, duty]
      );
      const duty = cubicSplineInterpolate(invertedPoints, rpm);
      return clamp(Math.round(duty), 0, 100);
    }
    return 0;
  }

  let showCalibration = false;
  function openCalibration() {
    showCalibration = true;
  }
  function closeCalibration() {
    showCalibration = false;
    // If user cancelled and we still don't have calibration, disable live
    if (!calibrationPoints) {
      showLive = false;
    }
  }
  async function handleCalibrationDone(pts: [number, number][]) {
    calibrationPoints = pts;
    await pollLiveOnce();
    success = "Calibrated";
    closeCalibration();
  }

  function toggleLive() {
    showLive = !showLive;
  }

  async function pollLiveOnce() {
    try {
      const res = await DefaultService.getThermal();
      if (!res.ok) return;
      const { temps, rpms } = parseThermalOutput(res.stdout);
      const t = pickTempForSensor(temps, sensor);
      if (t !== null) liveTemp = t;
      // Always update RPM - set to 0 if no RPM detected
      if (rpms.length) {
        const rpm = Math.max(...rpms);
        liveRpm = rpm;
      } else {
        liveRpm = 0; // No RPM detected, fan is stopped
      }
    } catch (_) {
      // Ignore transient errors
    }
  }

  let liveTimer: ReturnType<typeof setInterval> | null = null;
  function startLivePolling() {
    if (liveTimer) return;
    // Prime immediately, then interval
    pollLiveOnce();
    liveTimer = setInterval(pollLiveOnce, LIVE_POLL_MS);
  }
  function stopLivePolling() {
    if (liveTimer) {
      clearInterval(liveTimer);
      liveTimer = null;
    }
  }

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
  // Live crosshair coordinates
  $: liveDutyPct =
    liveRpm != null && calibrationPoints != null ? rpmToPercent(liveRpm) : null;
  $: liveX = liveTemp != null ? xToPx(liveTemp) : null;
  $: liveY = liveDutyPct != null ? yToPx(liveDutyPct) : null;

  onMount(async () => {
    try {
      const { ok, config } = await DefaultService.getConfig();
      if (ok && config) {
        // map backend mode to UI mode (accept lowercase or capitalized)
        const m = config.fan.mode;
        switch (m) {
          case "manual":
            mode = "Manual";
            break;
          case "curve":
            mode = "Curve";
            break;
          default:
            mode = "Auto";
            break;
        }
        if (config.fan.curve) {
          sensor = config.fan.curve.sensor;
          points = config.fan.curve.points.map(
            (p: any) => [p[0], p[1]] as Point
          );
          sortPointsInPlace();
          pollMs = config.fan.curve.poll_ms;
          hysteresisC = config.fan.curve.hysteresis_c;
          rateLimitPctPerStep = Math.max(
            1,
            config.fan.curve.rate_limit_pct_per_step
          );
        }
        if (config.fan.manual) {
          manualDutyPct = config.fan.manual.duty_pct;
        }
      }
    } catch (e: unknown) {
      error = e instanceof Error ? e.message : String(e);
    }
    // Load calibration from fan if present
    try {
      const { ok, config } = await DefaultService.getConfig();
      const cal = config?.fan?.calibration;
      if (cal?.points) calibrationPoints = cal.points as [number, number][];
    } catch (_) {}

    // Sync prevMode to whatever we loaded so lifting suppression won't trigger a save
    // Allow reactive saves after initial load completes
    prevMode = mode;
    onMountComplete = true;
  });

  onDestroy(() => {
    stopLivePolling();
  });

  const save = throttleDebounce(
    async () => {
      error = null;
      success = null;
      // Build minimal patch for new backend API
      const backendMode =
        mode === "Manual" ? "manual" : mode === "Curve" ? "curve" : "disabled";
      const fanPatch: PartialFanControlConfig = { mode: backendMode };
      if (backendMode === "manual") {
        fanPatch.manual = {
          duty_pct: clamp(manualDutyPct, 0, 100),
        };
      } else if (backendMode === "curve") {
        fanPatch.curve = {
          sensor,
          points: points.map((p) => [Math.round(p[0]), Math.round(p[1])]),
          poll_ms: Math.max(200, Math.floor(pollMs)),
          hysteresis_c: Math.max(0, Math.floor(hysteresisC)),
          rate_limit_pct_per_step: Math.max(1, Math.floor(rateLimitPctPerStep)),
        };
      }
      const patch: PartialConfig = { fan: fanPatch } as any;
      try {
        console.log("Saving Fan Control:", patch);
        const res = await DefaultService.setConfig(token, patch);
        if (!res.ok) throw new Error("Failed to save config");
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
  $: if (onMountComplete && mode !== prevMode) {
    prevMode = mode;
    save();
  }

  // While suppressed, keep prevMode in sync without saving
  $: if (!onMountComplete) {
    prevMode = mode;
  }

  // Start/stop live telemetry polling only in Curve mode
  $: if (mode === "Curve") startLivePolling();
  $: if (mode !== "Curve") stopLivePolling();

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

  function resetCurvePointsToDefaults() {
    points = DEFAULTS.curve.points;
    sortPointsInPlace();
    save();
  }

  function resetCurveSettingsToDefaults() {
    pollMs = DEFAULTS.curve.poll_ms;
    hysteresisC = DEFAULTS.curve.hysteresis_c;
    rateLimitPctPerStep = DEFAULTS.curve.rate_limit_pct_per_step;
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
                  on:click={resetCurveSettingsToDefaults}
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
                  class={`btn btn-xs btn-ghost opacity-90`}
                  on:click={toggleLive}
                  aria-label="Toggle live RPM overlay"
                  aria-pressed={showLive}
                  title="Live RPM"
                >
                  <Icon
                    icon="mdi:speedometer"
                    class={`text-base ${showLive ? "text-success" : ""}`}
                  />
                </button>
                <button
                  class="btn btn-xs btn-ghost"
                  on:click={() => (showSettings = true)}
                  aria-label="Open settings"
                >
                  <Icon icon="mdi:cog-outline" class="text-base" />
                </button>
                <button
                  class="btn btn-xs gap-1"
                  on:click={resetCurvePointsToDefaults}
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
                <defs>
                  <filter
                    id="live-glow"
                    x="-50%"
                    y="-50%"
                    width="200%"
                    height="200%"
                  >
                    <feGaussianBlur stdDeviation="2.2" result="coloredBlur" />
                    <feMerge>
                      <feMergeNode in="coloredBlur" />
                      <feMergeNode in="SourceGraphic" />
                    </feMerge>
                  </filter>
                </defs>
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

                {#if mode === "Curve" && showLive && calibrationPoints && liveX != null && liveY != null}
                  <!-- live crosshair -->
                  <g pointer-events="none">
                    <line
                      x1={padding.left}
                      y1={liveY}
                      x2={svgWidth - padding.right}
                      y2={liveY}
                      stroke="oklch(var(--a))"
                      stroke-width="1.25"
                      stroke-dasharray="4 3"
                      opacity="0.7"
                    />
                    <line
                      x1={liveX}
                      y1={padding.top}
                      x2={liveX}
                      y2={svgHeight - padding.bottom}
                      stroke="oklch(var(--a))"
                      stroke-width="1.25"
                      stroke-dasharray="4 3"
                      opacity="0.7"
                    />
                    <!-- live point + pulse ring (SVG-animate keeps center fixed) -->
                    <circle
                      cx={liveX}
                      cy={liveY}
                      r="5"
                      fill="oklch(var(--a))"
                      filter="url(#live-glow)"
                    />
                    <circle
                      cx={liveX}
                      cy={liveY}
                      r="6"
                      fill="none"
                      stroke="oklch(var(--a))"
                      stroke-width="2"
                      opacity="0.35"
                    >
                      <animate
                        attributeName="r"
                        values="6;14;6"
                        dur="1.4s"
                        repeatCount="indefinite"
                      />
                      <animate
                        attributeName="opacity"
                        values="0.35;0;0.35"
                        dur="1.4s"
                        repeatCount="indefinite"
                      />
                    </circle>
                  </g>
                {/if}
              </svg>
            </div>
            <div
              class="mt-2 flex items-center justify-between gap-2 text-xs opacity-70"
            >
              <div class="flex-1 min-w-0">
                Double‑click to add. Drag to adjust. Right‑click to delete.
              </div>
              <div
                class="ml-auto shrink-0 whitespace-nowrap flex items-center gap-2"
              >
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
                    min="1"
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
                    step="1"
                    class="range range-xs w-full"
                    bind:value={rateLimitPctPerStep}
                    on:input={save}
                  />
                </div>
              </div>

              <div class="form-control">
                <div class="flex items-center justify-between gap-2">
                  <div class="text-xs opacity-70">
                    Calibration aligns live RPM to duty curve.
                  </div>
                  <button
                    class="btn btn-xs"
                    on:click={openCalibration}
                    aria-label="Recalibrate fan"
                  >
                    Recalibrate
                  </button>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

{#if showCalibration}
  <CalibrationModal
    {token}
    on:done={(e) => handleCalibrationDone(e.detail)}
    on:cancel={closeCalibration}
  />
{/if}

<style>
  /* Removed CSS transform-based pulse to avoid SVG transform-origin issues */
</style>
