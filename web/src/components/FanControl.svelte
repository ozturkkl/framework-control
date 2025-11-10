<script lang="ts">
  import { onMount, onDestroy, tick } from "svelte";
  import { DefaultService } from "../api";
  import type { Config, PartialConfig, FanControlConfig } from "../api";
  import { throttleDebounce } from "../lib/utils";
  import { cubicSplineInterpolate } from "../lib/spline";
  import CalibrationModal from "./CalibrationModal.svelte";
  import UiSlider from "./UiSlider.svelte";
  import Icon from "@iconify/svelte";
  import MultiSelect from "./MultiSelect.svelte";
  import GraphPanel from "./GraphPanel.svelte";
  import { tooltip } from "../lib/tooltip";

  let error: string | null = null;
  let success: boolean | null = null;
  let successTimeout: ReturnType<typeof setTimeout> | null = null;
  let token = import.meta.env.VITE_CONTROL_TOKEN;

  // Live telemetry polling for current temperature and fan RPM
  const LIVE_POLL_MS = 1000;
  let liveTemp: number | null = null;
  let liveRpm: number | null = null;
  let calibrationPoints: [number, number][] | null = null;

  // Centralized defaults for the fan control config (backend schema)
  const DEFAULTS = {
    curve: {
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
  let selectedSensors: string[] = [];
  let availableSensors: string[] = [];
  let latestTemps: Record<string, number> = {};
  let selectedMaxSensor: string | null = null;

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
  // Tooltip state for selected point readout
  let pointCssX = 0;
  let pointCssY = 0;
  let selectedAnchorEl: SVGCircleElement | null = null;

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
    success = true;
    closeCalibration();
  }

  function toggleLive() {
    showLive = !showLive;
  }

  function pickTempForSelection(
    temps: Record<string, number>,
    selections: string[] | null
  ): number | null {
    let best: number | null = null;

    if (selections && selections.length > 0) {
      for (const s of selections) {
        const t = temps[s];
        if (!Number.isNaN(t)) {
          best = Math.max(best ?? 0, t);
        }
      }
    }
    return best;
  }

  async function pollLiveOnce() {
    try {
      const res = await DefaultService.getThermal();
      latestTemps = res.temps;
      const t = pickTempForSelection(latestTemps, selectedSensors);
      if (t !== null) liveTemp = t;
      liveRpm = Math.max(...res.rpms, 0);
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

  function updatePointTooltipPosition(idx: number) {
    if (!svgEl) return;
    const rect = svgEl.getBoundingClientRect();
    const scaleX = rect.width / svgWidth;
    const scaleY = rect.height / svgHeight;
    const p = points[idx];
    pointCssX = xToPx(p[0]) * scaleX;
    pointCssY = yToPx(p[1]) * scaleY;
  }
  $: if (selectedIdx != null) {
    updatePointTooltipPosition(selectedIdx);
  }

  onMount(async () => {
    const onDocumentPointerDown = (e: PointerEvent) => {
      const el = e.target as Element;
      if (!el.closest('[data-point="1"]')) {
        selectedIdx = null;
      }
    };
    document.addEventListener("pointerdown", onDocumentPointerDown, true);
    try {
      const config = await DefaultService.getConfig();
      if (config) {
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
          const sensors = config.fan.curve.sensors;
          if (sensors.length > 0) {
            selectedSensors = sensors;
          }
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
    // Load available sensors from thermal endpoint
    try {
      const t = await DefaultService.getThermal();
      availableSensors = Object.keys(t.temps);
      latestTemps = t.temps;
      // Best-effort: if user has no custom selection and list is empty, select all
      if (selectedSensors.length === 0 && availableSensors.length > 0) {
        selectedSensors = availableSensors.slice();
        save();
      }
    } catch (_) {}
    // Load calibration from fan if present
    try {
      const config = await DefaultService.getConfig();
      const cal = config?.fan?.calibration;
      if (cal?.points) calibrationPoints = cal.points as [number, number][];
    } catch (_) {}

    // Sync prevMode to whatever we loaded so lifting suppression won't trigger a save
    // Allow reactive saves after initial load completes
    prevMode = mode;
    onMountComplete = true;
    // Clean up on destroy
    onDestroy(() => {
      document.removeEventListener("pointerdown", onDocumentPointerDown, true);
    });
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
      const fanPatch: FanControlConfig = { mode: backendMode };
      if (backendMode === "manual") {
        fanPatch.manual = {
          duty_pct: clamp(manualDutyPct, 0, 100),
        };
      } else if (backendMode === "curve") {
        fanPatch.curve = {
          sensors: selectedSensors,
          points: points.map((p) => [Math.round(p[0]), Math.round(p[1])]),
          poll_ms: Math.max(200, Math.floor(pollMs)),
          hysteresis_c: Math.max(0, Math.floor(hysteresisC)),
          rate_limit_pct_per_step: Math.max(1, Math.floor(rateLimitPctPerStep)),
        };
      }
      const patch: PartialConfig = { fan: fanPatch } as any;
      try {
        console.log("Saving Fan Control:", patch);
        await DefaultService.setConfig(token, patch);
        if (successTimeout) {
          clearTimeout(successTimeout);
          successTimeout = null;
        }
        success = true;
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

  function startDrag(p: Point, ev: PointerEvent) {
    const idx = points.indexOf(p);
    if (idx === -1) return;
    selectedIdx = idx;
    isDragging = true;
    dragMoved = false;
    // Ensure keyboard focus on the point for accessibility
    (ev.currentTarget as HTMLElement | null)?.focus?.();
    (ev.target as Element).setPointerCapture?.(ev.pointerId);
    const rect = svgEl.getBoundingClientRect();
    const scaleX = svgWidth / rect.width;
    const scaleY = svgHeight / rect.height;
    const px = (ev.clientX - rect.left) * scaleX;
    const py = (ev.clientY - rect.top) * scaleY;
    const cx = xToPx(points[idx][0]);
    const cy = yToPx(points[idx][1]);
    dragOffset = { dx: px - cx, dy: py - cy };
    updatePointTooltipPosition(idx);
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
    points[idx][0] = Math.round(nx);
    points[idx][1] = Math.round(ny);
    points = points.slice();
    lastDragged = points[idx];
    updatePointTooltipPosition(idx);
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
      const found = points.indexOf(lastDragged);
      if (found !== -1) selectedIdx = found;
    }
    lastDragged = null;
    dragOffset = null;
  }

  async function onPointKeydown(pRef: Point, ev: KeyboardEvent) {
    const key = ev.key;
    if (
      key !== "ArrowLeft" &&
      key !== "ArrowRight" &&
      key !== "ArrowUp" &&
      key !== "ArrowDown" &&
      key !== "Home" &&
      key !== "End"
    ) {
      return;
    }
    ev.preventDefault();
    const idx = points.indexOf(pRef);
    if (idx === -1) return;
    const step = ev.ctrlKey ? 10 : ev.shiftKey ? 5 : 1;
    let [x, y] = points[idx];
    switch (key) {
      case "ArrowLeft":
        x = clamp(x - step, editableMinTemp, maxTemp);
        break;
      case "ArrowRight":
        x = clamp(x + step, editableMinTemp, maxTemp);
        break;
      case "ArrowUp":
        y = clamp(y + step, minDuty, maxDuty);
        break;
      case "ArrowDown":
        y = clamp(y - step, minDuty, maxDuty);
        break;
      case "Home":
        x = editableMinTemp;
        break;
      case "End":
        x = maxTemp;
        break;
    }
    const targetX = Math.round(x);
    const targetY = Math.round(y);
    // Mutate in place to preserve object identity for keyed each
    points[idx][0] = targetX;
    points[idx][1] = targetY;
    points = points.slice();
    // Preserve selection after sorting
    sortPointsInPlace();
    // Re-select by reference after sort so focus stays on the moved point
    const found = points.indexOf(pRef);
    if (found !== -1) {
      selectedIdx = found;
      updatePointTooltipPosition(found);
      await tick();
      const groups = svgEl?.querySelectorAll('g[data-point="1"]');
      const el = groups?.[found] as HTMLElement | undefined;
      el?.focus?.();
    }
    save();
  }

  async function addPointAt(ev: MouseEvent) {
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
    const newPoint: Point = [nx, ny];
    points.splice(insertIdx, 0, newPoint);
    points = points.slice();
    sortPointsInPlace();
    // Select and focus the newly created point
    const found = points.indexOf(newPoint);
    if (found !== -1) {
      selectedIdx = found;
      updatePointTooltipPosition(found);
      await tick();
      const groups = svgEl?.querySelectorAll('g[data-point="1"]');
      const el = groups?.[found] as HTMLElement | undefined;
      el?.focus?.();
    }
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

  function tempClass(t: number, selected: boolean) {
    if (!selected) return "opacity-50";
    if (t > 98) return "text-error";
    if (t > 90) return "text-warning";
    return "text-success";
  }

  // Track which selected sensor is currently the max
  $: (function computeSelectedMaxSensor() {
    let bestName: string | null = null;
    let best: number | null = null;
    for (const s of selectedSensors) {
      const t = latestTemps?.[s];
      if (typeof t === "number" && !Number.isNaN(t)) {
        if (best == null || t > best) {
          best = t;
          bestName = s;
        }
      }
    }
    selectedMaxSensor = bestName;
  })();
</script>

<svelte:window on:pointerup={endDrag} on:pointercancel={endDrag} />

<!-- preload icons -->
<div class="hidden">
  <Icon icon="mdi:speedometer-slow" />
  <Icon icon="mdi:thermometer-lines" />
  <Icon icon="mdi:timer-outline" />
  <Icon icon="mdi:fan" />
  <Icon icon="mdi:backup-restore" />
  <Icon icon="mdi:arrow-left" />
  <Icon icon="mdi:cog-outline" />
</div>

<!-- Overlay mode toggle positioned into the parent panel header area -->
<div
  class="absolute top-[0.62rem] left-36 right-14 flex items-center justify-start gap-2 text-sm"
>
  <div class="join border border-primary/35">
    <input
      type="radio"
      name="fan-mode"
      aria-label="Auto"
      class="btn btn-xs join-item"
      value="Auto"
      on:change={() => (mode = "Auto")}
      checked={mode === "Auto"}
    />
    <input
      type="radio"
      name="fan-mode"
      aria-label="Manual"
      class="btn btn-xs join-item"
      value="Manual"
      on:change={() => (mode = "Manual")}
      checked={mode === "Manual"}
    />
    <input
      type="radio"
      name="fan-mode"
      aria-label="Curve"
      class="btn btn-xs join-item"
      value="Curve"
      on:change={() => (mode = "Curve")}
      checked={mode === "Curve"}
    />
  </div>
  <span
    class="pointer-events-none select-none inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-500 text-white shadow transition duration-200 ease-out"
    style="opacity: {success ? 1 : 0}; transform: scale({success ? 1 : 0.9});"
    aria-hidden="true"
  >
    <Icon icon="mdi:check" class="text-base" />
  </span>
</div>

<div class="relative flex flex-col justify-center my-auto">
  {#if error}
    <div class="alert alert-error text-sm">
      <span>{error}</span>
    </div>
  {/if}

  {#if mode === "Auto"}
    <div class="text-md opacity-80 px-3 py-2">
      Fan will be controlled by the default firmware curve.
    </div>
  {/if}

  {#if mode === "Manual"}
    <UiSlider
      label="Manual duty"
      icon={"mdi:fan"}
      unit="%"
      min={0}
      max={100}
      step={1}
      hasEnabled={false}
      bind:value={manualDutyPct}
      on:input={save}
      on:change={save}
    />
  {/if}

  {#if mode === "Curve"}
    <GraphPanel>
      <svelte:fragment slot="top" let:openSettings>
        <div class="font-medium">
          <!-- Show current temp and live fan duty percentage -->
          <div class="flex items-center gap-2 ml-1">
            <span class="text-sm opacity-70">
              {latestTemps?.[selectedMaxSensor ?? ""]} °C • {rpmToPercent(
                liveRpm ?? 0
              )}%
            </span>
          </div>
        </div>
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
            on:click={openSettings}
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
      </svelte:fragment>

      <svelte:fragment slot="graph">
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
            <filter id="live-glow" x="-50%" y="-50%" width="200%" height="200%">
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
          {#each points as p, i (p)}
            <g
              on:pointerdown={(e) => startDrag(p, e)}
              on:contextmenu|preventDefault={() => deletePointAt(i)}
              on:focus={() => {
                selectedIdx = i;
                updatePointTooltipPosition(i);
              }}
              on:keydown={(e) => onPointKeydown(p, e)}
              class="cursor-pointer focus:outline-none focus-visible:outline-none"
              role="button"
              tabindex="0"
              data-point="1"
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

          {#if selectedIdx !== null}
            <!-- Invisible anchor circle bound for tooltip positioning -->
            <circle
              bind:this={selectedAnchorEl}
              cx={xToPx(points[selectedIdx][0])}
              cy={yToPx(points[selectedIdx][1])}
              r="1"
              opacity="0"
            />
          {/if}

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
                class="pulse-ring"
                fill="none"
                stroke="oklch(var(--a))"
                stroke-width="2"
              />
            </g>
          {/if}
        </svg>
        <!-- Minimal tooltip element rendered once; action portals and positions it -->
        <div
          use:tooltip={{
            anchor: () => selectedAnchorEl,
            visible: selectedIdx !== null,
          }}
          class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
        >
          {#if selectedIdx !== null}
            {points[selectedIdx][0]}°C · {points[selectedIdx][1]}%
          {/if}
        </div>
      </svelte:fragment>

      <svelte:fragment slot="bottom">
        <div class="flex items-center justify-between gap-2">
          <div class="flex-1 min-w-0 opacity-70 text-xs">
            Double‑click to add. Drag to adjust. Right‑click to delete.
          </div>
          <MultiSelect
            items={availableSensors}
            bind:selected={selectedSensors}
            label="Sensors"
            on:change={save}
          >
            <svelte:fragment slot="itemRight" let:item>
              {#if latestTemps?.[item] !== undefined}
                <span
                  class={`tabular-nums px-1.5 py-0.5 rounded-full border ${item === selectedMaxSensor ? "border-base-content/50 border-2" : "border-2 border-transparent"} ${tempClass(latestTemps[item], selectedSensors.includes(item))}`}
                >
                  {Math.round(latestTemps[item])} °C
                </span>
              {:else}
                <span class="opacity-60">—</span>
              {/if}
            </svelte:fragment>
          </MultiSelect>
        </div>
      </svelte:fragment>

      <svelte:fragment slot="settings-top-right">
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
            bind:value={pollMs}
            on:input={save}
          />

          <UiSlider
            label="Hysteresis"
            icon="mdi:thermometer-lines"
            unit="°C"
            min={1}
            max={10}
            step={1}
            bind:value={hysteresisC}
            on:input={save}
          />

          <UiSlider
            label="Rate limit per step"
            icon="mdi:speedometer-slow"
            unit="%"
            min={1}
            max={100}
            step={1}
            bind:value={rateLimitPctPerStep}
            on:input={save}
          />

          <div class="flex items-center justify-between gap-2 px-4 pb-3">
            <div class="text-xs opacity-70">
              Calibration aligns live RPM to duty curve.
            </div>
            <button
              class="btn btn-sm"
              on:click={openCalibration}
              aria-label="Recalibrate fan"
            >
              Recalibrate
            </button>
          </div>
        </div>
      </svelte:fragment>
    </GraphPanel>
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
  @keyframes pulseRing {
    0% {
      transform: scale(1);
      opacity: 0.35;
    }
    70% {
      opacity: 0;
    }
    100% {
      transform: scale(2);
      opacity: 0;
    }
  }
  .pulse-ring {
    transform-box: fill-box;
    transform-origin: center;
    animation: pulseRing 1.4s ease-out infinite;
    will-change: transform, opacity;
  }
  @media (prefers-reduced-motion: reduce) {
    .pulse-ring {
      animation: none;
      opacity: 0.25;
    }
  }
</style>
