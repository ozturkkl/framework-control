<script lang="ts">
    import { onMount, onDestroy, tick } from "svelte";
    import { DefaultService } from "../api";
    import type {
        FanControlConfig,
        FanOverride,
        CurveConfig,
        GlobalCurveConfig,
    } from "../api";
    import { configStore, followConfig, patch } from "../lib/config";
    import { throttleDebounce } from "../lib/utils";
    import { cubicSplineInterpolate } from "../lib/spline";
    import CalibrationModal from "./CalibrationModal.svelte";
    import UiControlCard from "./UiControlCard.svelte";
    import Icon from "@iconify/svelte";
    import MultiSelect from "./MultiSelect.svelte";
    import GraphPanel from "./GraphPanel.svelte";
    import FanSelector from "./FanSelector.svelte";
    import {
        PANEL_HEADER_OVERLAY_CLASS,
        PANEL_HEADER_TOGGLE_CLASS,
    } from "./Panel.svelte";
    import { tooltip } from "../lib/tooltip";
    import { measureSize, type MeasuredSize } from "../lib/measureSize";

    let error: string | null = null;

    // Live telemetry polling for current temperature and fan RPM
    const LIVE_POLL_MS = 1000;
    let liveTemp: number | null = null;
    let liveRpms: number[] = [];
    let calibrationByFan = new Map<number, [number, number][]>();
    $: hasCalibration = calibrationByFan.size > 0;

    // Centralized defaults for the fan control config (backend schema)
    type Point = [number, number];
    const DEFAULTS = {
        curve: {
            points: [
                [1, 30],
                [70, 30],
                [90, 50],
                [100, 80],
            ] as Point[],
            poll_ms: 500,
            hysteresis_c: 1,
            rate_limit_pct_per_step: 1,
            rate_limit_down_pct_per_step: 1,
        },
        manual: { duty_pct: 50 },
    };

    function curveConfigFromGlobal(g: GlobalCurveConfig): CurveConfig {
        const { poll_ms: _, ...curve } = g;
        return curve;
    }

    const FAN_MODES = ["Auto", "Manual", "Curve"] as const;
    let mode: (typeof FAN_MODES)[number] = "Auto";
    let onMountComplete = false;
    let prevMode: typeof mode = mode;
    let manualDutyPct = DEFAULTS.manual.duty_pct;

    // Curve editor state
    let points: Point[] = DEFAULTS.curve.points;
    let pollMs = DEFAULTS.curve.poll_ms;
    let hysteresisC = DEFAULTS.curve.hysteresis_c;
    let rateLimitPctPerStep = DEFAULTS.curve.rate_limit_pct_per_step;
    let rateLimitDownPctPerStep = DEFAULTS.curve.rate_limit_down_pct_per_step;
    let rateLimitDownEnabled = false;
    let selectedSensors: string[] = [];
    let availableSensors: string[] = [];
    let latestTemps: Record<string, number> = {};
    let selectedMaxSensor: string | null = null;

    let fanCount = 0;
    let fanNames: string[] = [];
    let activeFan: "all" | number = "all";
    let overrides: FanOverride[] = [];
    let loadingProfile = false;
    $: fanLabels = Array.from({ length: fanCount }, (_, i) => {
        const name = fanNames[i];
        const unique =
            !!name && fanNames.filter((n) => n === name).length === 1;
        return unique ? name : `${name} ${i + 1}`;
    });
    $: fanTabsVisible = fanCount > 1 && mode !== "Auto";
    $: modeOverrideFans = new Set(
        overrides
            .filter((o) =>
                mode === "Manual"
                    ? o.manual != null
                    : mode === "Curve"
                      ? o.curve != null
                      : false,
            )
            .map((o) => o.index),
    );
    let pollTipVisible = false;
    $: if (activeFan === "all") pollTipVisible = false;
    let downRateEnableBtn: HTMLButtonElement;
    let downRateEnableTipVisible = false;
    let downRateDisableBtn: HTMLButtonElement;
    let downRateDisableTipVisible = false;

    function probeColor(custom: boolean) {
        return custom ? "oklch(var(--a))" : "oklch(var(--p))";
    }

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
            if (onMountComplete && !hasCalibration) {
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
    function applyGraphSize(size: MeasuredSize) {
        svgWidth = size.width;
        svgHeight = size.height;
    }
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

    function pointsForFan(fanIndex: number | null): [number, number][] | null {
        if (fanIndex == null) return null;
        return calibrationByFan.get(fanIndex) ?? null;
    }

    function rpmToPercent(rpm: number, fanIndex: number | null = null): number {
        const pts = pointsForFan(fanIndex);
        if (pts) {
            const invertedPoints: [number, number][] = pts.map(
                ([duty, rpmVal]) => [rpmVal, duty],
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
        if (calibrationByFan.size === 0) {
            showLive = false;
        }
    }
    function applyCalibration(
        fans: { index: number; points: [number, number][] }[] | null | undefined,
    ) {
        const next = new Map<number, [number, number][]>();
        for (const fan of fans ?? []) {
            next.set(fan.index, fan.points);
        }
        calibrationByFan = next;
    }

    async function handleCalibrationDone(
        fans: { index: number; points: [number, number][] }[],
    ) {
        applyCalibration(fans);
        await pollLiveOnce();
        closeCalibration();
    }

    function toggleLive() {
        showLive = !showLive;
    }

    function pickTempForSelection(
        temps: Record<string, number>,
        selections: string[] | null,
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
            liveRpms = (res.fans ?? []).map((f) => f.rpm);
            fanNames = (res.fans ?? []).map((f) => f.name);
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

    $: liveRpm =
        liveRpms.length === 0
            ? null
            : activeFan === "all"
              ? Math.max(0, ...liveRpms)
              : (liveRpms[activeFan] ?? 0);

    function sortPointsInPlace() {
        points.sort((a, b) => a[0] - b[0]);
        points = points.slice();
    }

    function clamp(n: number, min: number, max: number) {
        return Math.max(min, Math.min(max, n));
    }

    function xToPx(x: number, width = svgWidth) {
        const w = width - padding.left - padding.right;
        return padding.left + ((x - minTemp) / (maxTemp - minTemp)) * w;
    }

    function yToPx(y: number, height = svgHeight) {
        const h = height - padding.top - padding.bottom;
        return padding.top + (1 - (y - minDuty) / (maxDuty - minDuty)) * h;
    }

    function pxToX(px: number, width = svgWidth) {
        const w = width - padding.left - padding.right;
        const t = clamp((px - padding.left) / w, 0, 1);
        return minTemp + t * (maxTemp - minTemp);
    }

    function pxToY(py: number, height = svgHeight) {
        const h = height - padding.top - padding.bottom;
        const t = clamp((py - padding.top) / h, 0, 1);
        return minDuty + (1 - t) * (maxDuty - minDuty);
    }

    function buildPath(pts: Point[], width = svgWidth, height = svgHeight) {
        if (!pts.length) return "";
        const segs = pts.map(
            (p, i) =>
                `${i === 0 ? "M" : "L"}${xToPx(p[0], width)},${yToPx(p[1], height)}`,
        );
        return segs.join(" ");
    }

    function buildArea(pts: Point[], width = svgWidth, height = svgHeight) {
        if (pts.length < 2) return "";
        const baseY = yToPx(0, height);
        const startX = xToPx(pts[0][0], width);
        const endX = xToPx(pts[pts.length - 1][0], width);
        const line = buildPath(pts, width, height);
        return `${line} L${endX},${baseY} L${startX},${baseY} Z`;
    }

    $: sortedPoints = [...points].sort((a, b) => a[0] - b[0]);
    $: sortedWithAnchors = ([[0, 0]] as Point[])
        .concat(sortedPoints)
        .concat([[100, 100]] as Point[]);
    $: pathLine = buildPath(sortedWithAnchors, svgWidth, svgHeight);
    $: pathArea = buildArea(sortedWithAnchors, svgWidth, svgHeight);
    // Live crosshair coordinates
    $: liveFanIndex =
        activeFan !== "all"
            ? activeFan
            : liveRpms.length === 0
              ? null
              : liveRpms.indexOf(Math.max(0, ...liveRpms));
    $: liveDutyPct =
        liveRpm != null && hasCalibration
            ? rpmToPercent(liveRpm, liveFanIndex)
            : null;
    $: liveX = liveTemp != null ? xToPx(liveTemp, svgWidth) : null;
    $: liveY = liveDutyPct != null ? yToPx(liveDutyPct, svgHeight) : null;

    // In "All" mode with per-fan overrides, a single probe can't represent
    // fans that follow different curves, so show one probe per fan instead.
    $: liveProbes = (() => {
        const width = svgWidth;
        const height = svgHeight;
        if (
            mode !== "Curve" ||
            !showLive ||
            !hasCalibration ||
            activeFan !== "all" ||
            !overrides.some((o) => o.curve != null)
        ) {
            return [];
        }
        const out = [];
        for (let i = 0; i < fanCount; i++) {
            const ov = overrides.find((o) => o.index === i);
            const sensors = ov?.curve?.sensors?.length
                ? ov.curve.sensors
                : selectedSensors;
            const t = pickTempForSelection(latestTemps, sensors);
            if (t == null) continue;
            const duty = rpmToPercent(liveRpms[i] ?? 0, i);
            out.push({
                i,
                label: fanLabels[i],
                x: xToPx(t, width),
                y: yToPx(duty, height),
                duty,
                custom: !!ov?.curve,
            });
        }
        return out;
    })();

    $: manualDutyReadouts = (() => {
        if (
            mode !== "Manual" ||
            activeFan !== "all" ||
            !overrides.some((o) => o.manual != null)
        ) {
            return [];
        }
        const out: {
            i: number;
            label: string;
            duty: number;
            custom: boolean;
        }[] = [];
        for (let i = 0; i < fanCount; i++) {
            const ov = overrides.find((o) => o.index === i);
            out.push({
                i,
                label: fanLabels[i],
                duty: ov?.manual?.duty_pct ?? manualDutyPct,
                custom: ov?.manual != null,
            });
        }
        return out;
    })();

    function updatePointTooltipPosition(
        idx: number,
        width = svgWidth,
        height = svgHeight,
    ) {
        if (!svgEl) return;
        const rect = svgEl.getBoundingClientRect();
        const scaleX = rect.width / width;
        const scaleY = rect.height / height;
        const p = points[idx];
        pointCssX = xToPx(p[0], width) * scaleX;
        pointCssY = yToPx(p[1], height) * scaleY;
    }
    $: if (selectedIdx != null) {
        updatePointTooltipPosition(selectedIdx, svgWidth, svgHeight);
    }

    onMount(async () => {
        try {
            const t = await DefaultService.getThermal();
            availableSensors = Object.keys(t.temps);
            latestTemps = t.temps;
            fanCount = t.fans?.length ?? 0;
            fanNames = (t.fans ?? []).map((f) => f.name);
        } catch (_) {}
        if (!$configStore.loaded && !$configStore.error) {
            await new Promise<void>((resolve) => {
                const unsub = configStore.subscribe((s) => {
                    if (s.loaded || s.error) {
                        unsub();
                        resolve();
                    }
                });
            });
        }
        if (
            $configStore.loaded &&
            selectedSensors.length === 0 &&
            availableSensors.length > 0
        ) {
            selectedSensors = availableSensors.slice();
            save();
        }
        onMountComplete = true;
    });

    // Clean up on destroy
    onDestroy(() => {
        stopLivePolling();
    });

    function readEditorCurveConfig(): CurveConfig {
        return {
            sensors: selectedSensors.slice(),
            points: points.map((p) => [p[0], p[1]]),
            hysteresis_c: hysteresisC,
            rate_limit_pct_per_step: rateLimitPctPerStep,
            ...(rateLimitDownEnabled
                ? { rate_limit_down_pct_per_step: rateLimitDownPctPerStep }
                : {}),
        };
    }

    function readEditorGlobalCurve() {
        return { ...readEditorCurveConfig(), poll_ms: pollMs };
    }

    function applyCurveConfig(c: CurveConfig, applySensors = true) {
        points = c.points.map((p) => [p[0], p[1]]) as Point[];
        sortPointsInPlace();
        hysteresisC = c.hysteresis_c;
        rateLimitPctPerStep = Math.max(1, c.rate_limit_pct_per_step);
        rateLimitDownEnabled = c.rate_limit_down_pct_per_step != null;
        rateLimitDownPctPerStep = Math.max(
            1,
            c.rate_limit_down_pct_per_step ?? c.rate_limit_pct_per_step,
        );
        if (applySensors) selectedSensors = c.sensors.slice();
    }

    function applyGlobalCurveConfig(c: GlobalCurveConfig, applySensors = true) {
        applyCurveConfig(c, applySensors);
        pollMs = c.poll_ms;
    }

    function applyLoadedGlobalConfig(fan: FanControlConfig) {
        if (fan.curve) {
            applyGlobalCurveConfig(
                fan.curve,
                fan.curve.sensors.length > 0,
            );
        }
        if (fan.manual) manualDutyPct = fan.manual.duty_pct;
    }

    function applyEditorFromFan(
        fan: FanControlConfig | undefined,
        target: "all" | number,
    ) {
        if (target === "all") {
            if (fan) applyLoadedGlobalConfig(fan);
            return;
        }
        const ov = overrides.find((o) => o.index === target);
        const globalCurve = fan?.curve;
        const globalManual = fan?.manual;
        if (ov?.curve) {
            applyCurveConfig(ov.curve);
        } else if (globalCurve) {
            applyCurveConfig(curveConfigFromGlobal(globalCurve));
        }
        manualDutyPct =
            ov?.manual?.duty_pct ??
            globalManual?.duty_pct ??
            DEFAULTS.manual.duty_pct;
        // Poll interval is a single control-loop cadence shared by all fans.
        pollMs = globalCurve?.poll_ms ?? DEFAULTS.curve.poll_ms;
    }

    function applyFanConfig(fan: FanControlConfig) {
        switch (fan.mode) {
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
        if (mode === "Auto") {
            activeFan = "all";
        }
        overrides = fan.overrides ?? [];
        applyCalibration(
            fan.calibration?.fans as
                | { index: number; points: [number, number][] }[]
                | undefined,
        );
        applyEditorFromFan(fan, activeFan);
        prevMode = mode;
    }

    onDestroy(followConfig({ select: (c) => c.fan, apply: applyFanConfig }));

    function upsertOverride(
        list: FanOverride[],
        item: FanOverride,
    ): FanOverride[] {
        const out = list.filter((o) => o.index !== item.index);
        out.push(item);
        out.sort((a, b) => a.index - b.index);
        return out;
    }

    // Write the active fan tab's editor values into overrides if we're on a fan tab.
    function commitOverrideFromEditor() {
        if (loadingProfile || activeFan === "all") return;
        const idx = activeFan;
        const existing = overrides.find((o) => o.index === idx);
        const next: FanOverride = { ...(existing ?? {}), index: idx };
        if (mode === "Manual") {
            next.manual = { duty_pct: manualDutyPct };
        } else if (mode === "Curve") {
            next.curve = readEditorCurveConfig();
        }
        overrides = upsertOverride(overrides, next);
    }

    async function selectFan(target: "all" | number) {
        if (target === activeFan) return;
        loadingProfile = true;
        try {
            if (activeFan === "all") {
                // Flush global config before switching tabs.
                await doSave({ silent: true });
            } else {
                commitOverrideFromEditor();
            }
            activeFan = target;
            selectedIdx = null;
            applyEditorFromFan($configStore.config?.fan, target);
            await tick();
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : String(e);
        } finally {
            loadingProfile = false;
        }
    }

    function clearOverride(idx: number) {
        overrides = overrides
            .map((o) => {
                if (o.index !== idx) return o;
                const next = { ...o };
                if (mode === "Manual") delete next.manual;
                else if (mode === "Curve") delete next.curve;
                return next;
            })
            .filter((o) => o.manual != null || o.curve != null);
        if (activeFan === idx) selectFan("all");
        save();
    }

    async function doSave(_opts?: { silent?: boolean }) {
        error = null;
        const backendMode =
            mode === "Manual"
                ? "manual"
                : mode === "Curve"
                  ? "curve"
                  : "disabled";
        const fanPatch: FanControlConfig = { mode: backendMode };
        if (activeFan === "all") {
            if (backendMode === "manual") {
                fanPatch.manual = {
                    duty_pct: clamp(manualDutyPct, 0, 100),
                };
            } else if (backendMode === "curve") {
                fanPatch.curve = readEditorGlobalCurve();
            }
        }
        // Overrides replace wholesale when provided; always send the full list.
        fanPatch.overrides = overrides;
        try {
            await patch({ fan: fanPatch });
        } catch (e: unknown) {
            error = e instanceof Error ? e.message : String(e);
        }
    }

    const save = throttleDebounce(doSave, 200, false, true);

    // Apply local mode changes
    $: if (onMountComplete && mode !== prevMode) {
        prevMode = mode;
        if (mode === "Auto" && activeFan !== "all") {
            selectFan("all");
        }
        save();
    }

    // While suppressed, keep prevMode in sync without saving
    $: if (!onMountComplete) {
        prevMode = mode;
    }

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
        const cx = xToPx(points[idx][0], svgWidth);
        const cy = yToPx(points[idx][1], svgHeight);
        dragOffset = { dx: px - cx, dy: py - cy };
        updatePointTooltipPosition(idx, svgWidth, svgHeight);
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
        const nx = clamp(pxToX(px, svgWidth), editableMinTemp, maxTemp);
        const ny = clamp(pxToY(py, svgHeight), minDuty, maxDuty);
        points[idx][0] = Math.round(nx);
        points[idx][1] = Math.round(ny);
        points = points.slice();
        lastDragged = points[idx];
        updatePointTooltipPosition(idx, svgWidth, svgHeight);
        commitOverrideFromEditor();
        save();
    }

    function endDrag(ev: PointerEvent) {
        if (!isDragging) return;
        isDragging = false;
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
            updatePointTooltipPosition(found, svgWidth, svgHeight);
            await tick();
            const groups = svgEl?.querySelectorAll('g[data-point="1"]');
            const el = groups?.[found] as HTMLElement | undefined;
            el?.focus?.();
        }
        commitOverrideFromEditor();
        save();
    }

    async function addPointAt(ev: MouseEvent) {
        // Use double click to avoid conflict with drags
        const rect = svgEl.getBoundingClientRect();
        const scaleX = svgWidth / rect.width;
        const scaleY = svgHeight / rect.height;
        const px = (ev.clientX - rect.left) * scaleX;
        const py = (ev.clientY - rect.top) * scaleY;
        const nx = clamp(
            Math.round(pxToX(px, svgWidth)),
            editableMinTemp,
            maxTemp,
        );
        const ny = clamp(Math.round(pxToY(py, svgHeight)), minDuty, maxDuty);
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
            updatePointTooltipPosition(found, svgWidth, svgHeight);
            await tick();
            const groups = svgEl?.querySelectorAll('g[data-point="1"]');
            const el = groups?.[found] as HTMLElement | undefined;
            el?.focus?.();
        }
        commitOverrideFromEditor();
        save();
    }

    function deletePointAt(index: number) {
        points.splice(index, 1);
        points = points.slice();
        if (selectedIdx === index) selectedIdx = null;
        commitOverrideFromEditor();
        save();
    }

    function resetCurvePointsToDefaults() {
        if (activeFan !== "all") return clearOverride(activeFan);
        points = DEFAULTS.curve.points;
        sortPointsInPlace();
        commitOverrideFromEditor();
        save();
    }

    function resetCurveSettingsToDefaults() {
        if (activeFan !== "all") return clearOverride(activeFan);
        pollMs = DEFAULTS.curve.poll_ms;
        hysteresisC = DEFAULTS.curve.hysteresis_c;
        rateLimitPctPerStep = DEFAULTS.curve.rate_limit_pct_per_step;
        rateLimitDownPctPerStep =
            DEFAULTS.curve.rate_limit_down_pct_per_step;
        rateLimitDownEnabled = false;
        commitOverrideFromEditor();
        save();
    }

    function toggleDownRate() {
        rateLimitDownEnabled = !rateLimitDownEnabled;
        commitOverrideFromEditor();
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

<div class={PANEL_HEADER_OVERLAY_CLASS}>
    <div
        class={PANEL_HEADER_TOGGLE_CLASS}
        role="radiogroup"
        aria-label="Fan mode"
    >
        {#each FAN_MODES as value (value)}
            <input
                type="radio"
                name="fan-mode"
                aria-label={value}
                class="btn btn-xs join-item"
                {value}
                bind:group={mode}
            />
        {/each}
    </div>
</div>

<!-- preload icons -->
<div class="hidden">
    <Icon icon="mdi:speedometer-slow" />
    <Icon icon="mdi:thermometer-lines" />
    <Icon icon="mdi:timer-outline" />
    <Icon icon="mdi:fan" />
    <Icon icon="mdi:backup-restore" />
    <Icon icon="mdi:arrow-left" />
    <Icon icon="mdi:cog-outline" />
    <Icon icon="mdi:close" />
    <Icon icon="mdi:plus" />
    <Icon icon="mdi:call-split" />
    <Icon icon="mdi:call-merge" />
</div>

<div class="relative flex flex-col h-full min-h-0">
    {#if error}
        <div class="alert alert-error text-sm">
            <span>{error}</span>
        </div>
    {/if}

    {#if mode === "Auto"}
        <div class="flex-1 min-h-0 flex flex-col">
            <div
                class="card bg-base-200 p-3 h-full min-h-0 flex flex-col flex-1 w-full"
            >
                <div
                    class="flex-1 min-h-0 flex flex-col items-center justify-center px-6"
                >
                    <div class="text-center">
                        <div
                            class="font-medium tracking-tight text-5xl leading-none"
                        >
                            Auto
                        </div>
                        <div class="mt-2 text-xs opacity-60">
                            Default firmware curve
                        </div>
                    </div>
                </div>
            </div>
        </div>
    {/if}

    {#if mode === "Manual"}
        <div class="flex-1 min-h-0 flex flex-col">
            <UiControlCard
                label="Manual duty"
                unit="%"
                min={0}
                max={100}
                step={1}
                bind:value={manualDutyPct}
                on:input={() => {
                    commitOverrideFromEditor();
                    save();
                }}
                on:change={() => {
                    commitOverrideFromEditor();
                    save();
                }}
            >
                <div
                    slot="header-trailing"
                    class="flex items-center gap-1.5 min-w-0"
                >
                    {#if manualDutyReadouts.length > 0}
                        <div class="font-medium min-w-0 overflow-hidden">
                            <div
                                class="flex items-center gap-1.5 whitespace-nowrap overflow-hidden"
                            >
                                {#each manualDutyReadouts as probe, i (probe.i)}
                                    {#if i > 0}
                                        <span class="opacity-60">·</span>
                                    {/if}
                                    <span
                                        class="text-xs opacity-80 tabular-nums inline-flex items-center gap-1"
                                    >
                                        <span
                                            class="inline-block w-1.5 h-1.5 rounded-full"
                                            style={`background:${probeColor(probe.custom)}`}
                                        ></span>
                                        <span
                                            >{probe.label} {probe.duty}%</span
                                        >
                                    </span>
                                {/each}
                            </div>
                        </div>
                    {/if}
                    {#if fanTabsVisible}
                        <FanSelector
                            {activeFan}
                            {fanLabels}
                            overrideFans={modeOverrideFans}
                            on:select={(e) => selectFan(e.detail)}
                            on:clear={(e) => clearOverride(e.detail)}
                        />
                    {/if}
                </div>
            </UiControlCard>
        </div>
    {/if}

    {#if mode === "Curve"}
        <div class="flex-1 min-h-0 flex flex-col">
            <GraphPanel>
            <svelte:fragment slot="top" let:openSettings>
                <div class="flex items-center gap-1.5 min-w-0 flex-1">
                    {#if fanTabsVisible}
                        <FanSelector
                            {activeFan}
                            {fanLabels}
                            overrideFans={modeOverrideFans}
                            on:select={(e) => selectFan(e.detail)}
                            on:clear={(e) => clearOverride(e.detail)}
                        />
                    {/if}
                    <div class="font-medium min-w-0 overflow-hidden">
                        <div
                            class="flex items-center gap-1.5 whitespace-nowrap overflow-hidden"
                        >
                            {#if liveProbes.length > 0}
                                {#each liveProbes as probe, i (probe.i)}
                                    {#if i > 0}
                                        <span class="opacity-60">·</span>
                                    {/if}
                                    <span
                                        class="text-xs opacity-80 tabular-nums inline-flex items-center gap-1"
                                    >
                                        <span
                                            class="inline-block w-1.5 h-1.5 rounded-full"
                                            style={`background:${probeColor(probe.custom)}`}
                                        ></span>
                                        <span>{probe.label} {probe.duty}%</span>
                                    </span>
                                {/each}
                            {:else}
                                <span class="text-sm opacity-70">
                                    {latestTemps?.[selectedMaxSensor ?? ""]} °C • {rpmToPercent(
                                        liveRpm ?? 0,
                                        liveFanIndex,
                                    )}%
                                </span>
                            {/if}
                        </div>
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
                <div
                    class="relative w-full h-full min-h-[220px]"
                    use:measureSize={{ onChange: applyGraphSize }}
                >
                <svg
                    bind:this={svgEl}
                    class="absolute inset-0 w-full h-full touch-none select-none bg-base-100 rounded border border-base-300"
                    viewBox={`0 0 ${svgWidth} ${svgHeight}`}
                    preserveAspectRatio="none"
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
                            <feGaussianBlur
                                stdDeviation="2.2"
                                result="coloredBlur"
                            />
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
                            y1={yToPx(0, svgHeight)}
                            x2={svgWidth - padding.right}
                            y2={yToPx(0, svgHeight)}
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
                    {#each [0, 20, 40, 60, 80, 100] as d (d)}
                        <g>
                            <line
                                x1={padding.left}
                                y1={yToPx(d, svgHeight)}
                                x2={svgWidth - padding.right}
                                y2={yToPx(d, svgHeight)}
                                stroke="currentColor"
                                class="opacity-10"
                            />
                            <text
                                x={padding.left - 6}
                                y={yToPx(d, svgHeight) + 4}
                                text-anchor="end"
                                class="fill-current opacity-60 text-[10px]"
                                >{d}%</text
                            >
                        </g>
                    {/each}
                    {#each [20, 40, 60, 80, 100] as t (t)}
                        <g>
                            <line
                                x1={xToPx(t, svgWidth)}
                                y1={padding.top}
                                x2={xToPx(t, svgWidth)}
                                y2={svgHeight - padding.bottom}
                                stroke="currentColor"
                                class="opacity-10"
                            />
                            <text
                                x={xToPx(t, svgWidth)}
                                y={svgHeight - padding.bottom + 16}
                                text-anchor="middle"
                                class="fill-current opacity-60 text-[10px]"
                                >{t}°C</text
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
                            on:contextmenu|preventDefault={() =>
                                deletePointAt(i)}
                            on:focus={() => {
                                selectedIdx = i;
                                updatePointTooltipPosition(
                                    i,
                                    svgWidth,
                                    svgHeight,
                                );
                            }}
                            on:keydown={(e) => onPointKeydown(p, e)}
                            class="cursor-pointer focus:outline-none focus-visible:outline-none"
                            role="button"
                            tabindex="0"
                            data-point="1"
                            aria-label={`Point at ${p[0]}°C ${p[1]}%`}
                        >
                            <circle
                                cx={xToPx(p[0], svgWidth)}
                                cy={yToPx(p[1], svgHeight)}
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
                            cx={xToPx(points[selectedIdx][0], svgWidth)}
                            cy={yToPx(points[selectedIdx][1], svgHeight)}
                            r="1"
                            opacity="0"
                        />
                    {/if}

                    {#if mode === "Curve" && showLive && hasCalibration && liveX != null && liveY != null && liveProbes.length === 0}
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

                    {#if liveProbes.length > 0}
                        <!-- One probe per fan: custom fans (accent) sit off the
                             shared curve, fans following the global curve (primary)
                             land on the line. -->
                        <g pointer-events="none">
                            {#each liveProbes as probe (probe.i)}
                                <circle
                                    cx={probe.x}
                                    cy={probe.y}
                                    r="5"
                                    fill={probeColor(probe.custom)}
                                    filter="url(#live-glow)"
                                />
                                <text
                                    x={probe.x}
                                    y={probe.y - 9}
                                    text-anchor="middle"
                                    class="fill-current text-[9px] font-medium"
                                    opacity="0.8"
                                >
                                    {probe.i + 1}
                                </text>
                            {/each}
                        </g>
                    {/if}
                </svg>
                <!-- Minimal tooltip element rendered once; action portals and positions it -->
                <div
                    use:tooltip={{
                        anchor: () => selectedAnchorEl,
                        visible: selectedIdx !== null,
                        onDismiss: () => (selectedIdx = null),
                    }}
                    class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
                >
                    {#if selectedIdx !== null}
                        {points[selectedIdx][0]}°C · {points[selectedIdx][1]}%
                    {/if}
                </div>
                </div>
            </svelte:fragment>

            <svelte:fragment slot="bottom">
                <div class="flex items-center justify-between gap-2">
                    <div class="flex-1 min-w-0 opacity-70 text-xs">
                        Double‑click to add. Drag to adjust. Right‑click to
                        delete.
                    </div>
                    <MultiSelect
                        items={availableSensors}
                        bind:selected={selectedSensors}
                        label="Sensors"
                        on:change={() => {
                            commitOverrideFromEditor();
                            save();
                        }}
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
                    <div
                        role="presentation"
                        on:mousemove={() => {
                            if (activeFan !== "all") pollTipVisible = true;
                        }}
                        on:mouseleave={() => (pollTipVisible = false)}
                    >
                        <UiControlCard
                            label="Poll interval"
                            icon="mdi:timer-outline"
                            unit="ms"
                            min={500}
                            max={5000}
                            step={100}
                            disabled={activeFan !== "all"}
                            bind:value={pollMs}
                            on:input={() => {
                                commitOverrideFromEditor();
                                save();
                            }}
                        />
                    </div>
                    <div
                        use:tooltip={{
                            visible: pollTipVisible,
                            followMouse: true,
                            attachGlobalDismiss: false,
                        }}
                        class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
                    >
                        Poll interval can’t be set per fan — it’s shared by all
                        fans.
                    </div>

                    <UiControlCard
                        label="Hysteresis"
                        icon="mdi:thermometer-lines"
                        unit="°C"
                        min={1}
                        max={10}
                        step={1}
                        bind:value={hysteresisC}
                        on:input={() => {
                            commitOverrideFromEditor();
                            save();
                        }}
                    />

                    <UiControlCard
                        label={rateLimitDownEnabled
                            ? "Rate limit (speed up)"
                            : "Rate limit per step"}
                        icon="mdi:speedometer-slow"
                        unit="%"
                        min={1}
                        max={100}
                        step={1}
                        bind:value={rateLimitPctPerStep}
                        on:input={() => {
                            commitOverrideFromEditor();
                            save();
                        }}
                    >
                        <div
                            slot="label-trailing"
                            class="relative"
                            class:hidden={rateLimitDownEnabled}
                        >
                            <button
                                class="btn btn-ghost btn-xs btn-square"
                                aria-label="Set a separate spin-down rate"
                                bind:this={downRateEnableBtn}
                                on:mouseenter={() =>
                                    (downRateEnableTipVisible = true)}
                                on:mouseleave={() =>
                                    (downRateEnableTipVisible = false)}
                                on:focus={() =>
                                    (downRateEnableTipVisible = true)}
                                on:blur={() =>
                                    (downRateEnableTipVisible = false)}
                                on:click={toggleDownRate}
                            >
                                <Icon
                                    icon="mdi:call-split"
                                    class="w-4 h-4"
                                />
                            </button>
                            <div
                                use:tooltip={{
                                    anchor: downRateEnableBtn,
                                    visible: downRateEnableTipVisible,
                                    attachGlobalDismiss: false,
                                }}
                                class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
                            >
                                Separate spin-down rate
                            </div>
                        </div>
                    </UiControlCard>

                    {#if rateLimitDownEnabled}
                        <UiControlCard
                            label="Rate limit (speed down)"
                            icon="mdi:speedometer-slow"
                            unit="%"
                            min={1}
                            max={100}
                            step={1}
                            bind:value={rateLimitDownPctPerStep}
                            on:input={() => {
                                commitOverrideFromEditor();
                                save();
                            }}
                        >
                            <div slot="label-trailing" class="relative">
                                <button
                                    class="btn btn-ghost btn-xs btn-square"
                                    aria-label="Use one rate for both directions"
                                    bind:this={downRateDisableBtn}
                                    on:mouseenter={() =>
                                        (downRateDisableTipVisible = true)}
                                    on:mouseleave={() =>
                                        (downRateDisableTipVisible = false)}
                                    on:focus={() =>
                                        (downRateDisableTipVisible = true)}
                                    on:blur={() =>
                                        (downRateDisableTipVisible = false)}
                                    on:click={toggleDownRate}
                                >
                                    <Icon
                                        icon="mdi:call-merge"
                                        class="w-4 h-4"
                                    />
                                </button>
                                <div
                                    use:tooltip={{
                                        anchor: downRateDisableBtn,
                                        visible: downRateDisableTipVisible,
                                        attachGlobalDismiss: false,
                                    }}
                                    class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
                                >
                                    One rate for both directions
                                </div>
                            </div>
                        </UiControlCard>
                    {/if}

                    <div
                        class="flex items-center justify-between gap-2 px-4 pb-3"
                    >
                        <div class="text-xs opacity-70">
                            Calibration aligns live RPM to duty curve.
                        </div>
                        <button
                            class="btn btn-sm"
                            on:click={openCalibration}
                            aria-label="Recalibrate fans"
                        >
                            Recalibrate
                        </button>
                    </div>
                </div>
            </svelte:fragment>
            </GraphPanel>
        </div>
    {/if}
</div>

{#if showCalibration}
    <CalibrationModal
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
