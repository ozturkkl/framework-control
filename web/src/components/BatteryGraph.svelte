<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import Icon from '@iconify/svelte';
	import { DefaultService, type BatterySample } from '../api';
	import UiControlCard from './UiControlCard.svelte';
	import GraphPanel from './GraphPanel.svelte';
	import { tooltip } from '../lib/tooltip';
	import { followConfig, patch } from '../lib/config';
	import { measureSize, type MeasuredSize } from '../lib/measureSize';
	import { bracketByTime, pointerToDomainX } from '../lib/plot';

	const WINDOW_KEY = 'fc.battery.window';
	const WINDOW_SINCE_CHARGE = 'Since last charge';
	const WINDOW_SECONDS: Record<string, number> = {
		'1 hour': 3600,
		'6 hours': 21600,
		'12 hours': 43200,
		'24 hours': 86400,
		'2 days': 2 * 86400,
		'7 days': 7 * 86400,
	};
	const WINDOW_OPTIONS = [WINDOW_SINCE_CHARGE, ...Object.keys(WINDOW_SECONDS)];
	const MIN_POLL_SECS = 5;
	const MAX_POLL_SECS = 60;
	const GAP_MS = 2.1 * MAX_POLL_SECS * 1000;
	const DEFAULT_POLL_SECS = 15;
	const CHARGE_COLOR = '#22c55e';
	const POWER_COLOR = '#3b82f6';

	let pollSecs = DEFAULT_POLL_SECS;
	let windowChoice = WINDOW_SINCE_CHARGE;
	let historyTimer: ReturnType<typeof setInterval> | null = null;
	let samples: BatterySample[] = [];
	let nowMs = Date.now();
	let historyError: string | null = null;

	const padding = { left: 36, right: 44, top: 12, bottom: 22 };
	let svgWidth = 400;
	let svgHeight = 220;
	let svgEl: SVGSVGElement;
	function applyGraphSize(size: MeasuredSize) {
		svgWidth = size.width;
		svgHeight = size.height;
	}
	let hoverCircleEl: SVGCircleElement | null = null;

	type Pt = [number, number];
	type HoverInfo = {
		ts: number;
		chargePct?: number;
		watts?: number;
		anchorY: number;
		anchor: 'charge' | 'power';
	} | null;
	let hover: HoverInfo = null;

	$: tMax = samples.length ? samples[samples.length - 1].ts_ms : nowMs;
	$: chargeStartMs = lastChargeStartMs(samples);
	$: requestedMin = windowStartMs(windowChoice, samples, nowMs, chargeStartMs);
	$: tMin = samples.length ? Math.max(requestedMin, samples[0].ts_ms) : requestedMin;
	$: includeDate = tMax - tMin >= 20 * 3600 * 1000 || new Date(tMin).toDateString() !== new Date(tMax).toDateString();
	$: windowed = samples.filter((s) => s.ts_ms >= tMin && s.ts_ms <= tMax);
	$: chargeRaw = (() => {
		const pts = windowed
			.filter((s) => s.charge_pct != null)
			.map((s) => [s.ts_ms, s.charge_pct as number] as Pt);
		const prior = gapAnchorBefore(samples, pts[0]?.[0], (s) => s.charge_pct != null);
		return prior && pts.length ? [[prior.ts_ms, prior.charge_pct as number] as Pt, ...pts] : pts;
	})();
	$: wattRaw = windowed.filter((s) => s.watts != null).map((s) => [s.ts_ms, s.watts as number] as Pt);
	$: chargeSegs = splitByTime(chargeRaw, GAP_MS).map((seg) => decimate(seg, 400));
	$: wattSegs = splitByTime(wattRaw, GAP_MS).map((seg) => decimate(seg, 400));
	$: chargeGaps = connectors(chargeSegs);
	$: wattScale = wattsScale(wattRaw.map((p) => p[1]));
	$: last = windowed.length ? windowed[windowed.length - 1] : undefined;
	// Same lead-in as chargeRaw so hover can lerp charge across a window that starts mid-gap.
	$: hoverSamples = (() => {
		const prior = gapAnchorBefore(samples, windowed[0]?.ts_ms);
		return prior ? [prior, ...windowed] : windowed;
	})();
	$: plotSeries = [
		{ key: 'charge', color: CHARGE_COLOR, segs: chargeSegs, gaps: chargeGaps, yToPx: yToPxPct },
		// Power is instantaneous — leave sampling gaps empty rather than inventing values.
		{ key: 'power', color: POWER_COLOR, segs: wattSegs, gaps: [], yToPx: yToPxWatts },
	];

	function pickStep(candidates: number[], approx: number): number {
		for (const c of candidates) {
			if (c >= approx) return c;
		}
		return candidates[candidates.length - 1];
	}

	/** Last sample before `afterTs` when that span is a sampling gap (for clipped dotted lead-in). */
	function gapAnchorBefore(
		list: BatterySample[],
		afterTs: number | undefined,
		ok?: (s: BatterySample) => boolean,
	): BatterySample | null {
		if (afterTs == null) return null;
		for (let i = list.length - 1; i >= 0; i--) {
			const s = list[i];
			if (s.ts_ms >= afterTs) continue;
			if (ok && !ok(s)) continue;
			return afterTs - s.ts_ms > GAP_MS ? s : null;
		}
		return null;
	}

	function splitByTime(points: Pt[], gapMs: number): Pt[][] {
		if (!points.length) return [];
		const segs: Pt[][] = [[points[0]]];
		for (let i = 1; i < points.length; i++) {
			if (points[i][0] - points[i - 1][0] > gapMs) segs.push([]);
			segs[segs.length - 1].push(points[i]);
		}
		return segs;
	}

	function decimate(points: Pt[], maxPoints: number): Pt[] {
		if (points.length <= maxPoints) return points;
		const stride = Math.ceil(points.length / maxPoints);
		const out: Pt[] = [];
		for (let i = 0; i < points.length; i += stride) out.push(points[i]);
		const lastPt = points[points.length - 1];
		if (out[out.length - 1] !== lastPt) out.push(lastPt);
		return out;
	}

	function lastChargeStartMs(list: BatterySample[]): number | null {
		if (!list.length) return null;
		let start: number | null = null;
		let prevOn = false;
		for (const s of list) {
			const on = s.ac_present === true;
			if (on && !prevOn) start = s.ts_ms;
			prevOn = on;
		}
		return start;
	}

	function windowStartMs(choice: string, list: BatterySample[], now: number, chargeAt: number | null): number {
		const secs = WINDOW_SECONDS[choice];
		if (secs != null) return now - secs * 1000;
		if (chargeAt != null) return chargeAt;
		if (list.length) return list[0].ts_ms;
		return now;
	}

	function xToPx(x: number, width = svgWidth) {
		if (tMax === tMin) return padding.left;
		const w = width - padding.left - padding.right;
		return padding.left + ((x - tMin) / (tMax - tMin)) * w;
	}
	function yToPxPct(y: number, height = svgHeight) {
		const h = height - padding.top - padding.bottom;
		return padding.top + (1 - y / 100) * h;
	}
	function yToPxWatts(y: number, height = svgHeight) {
		const h = height - padding.top - padding.bottom;
		const { min, max } = wattScale;
		if (max === min) return padding.top + h / 2;
		return padding.top + (1 - (y - min) / (max - min)) * h;
	}

	function buildPath(
		points: Pt[],
		yToPx: (y: number, height?: number) => number,
		width = svgWidth,
		height = svgHeight,
	) {
		return points
			.map((p, i) => `${i === 0 ? 'M' : 'L'}${xToPx(p[0], width)},${yToPx(p[1], height)}`)
			.join(' ');
	}

	function connectors(segs: Pt[][]): Array<[Pt, Pt]> {
		const out: Array<[Pt, Pt]> = [];
		for (let i = 0; i < segs.length - 1; i++) {
			const a = segs[i][segs[i].length - 1];
			const b = segs[i + 1][0];
			if (a && b) out.push([a, b]);
		}
		return out;
	}

	function wattsScale(values: number[]): { min: number; max: number; ticks: number[] } {
		let lo = 0;
		let hi = 0;
		for (const v of values) {
			if (Number.isFinite(v)) {
				lo = Math.min(lo, v);
				hi = Math.max(hi, v);
			}
		}
		const pad = Math.max(4, (hi - lo) * 0.15);
		if (lo < 0) lo -= pad;
		if (hi > 0) hi += pad;
		lo = Math.min(0, lo);
		hi = Math.max(0, hi);
		if (lo === 0 && hi === 0) hi = 8;
		const span = Math.max(8, hi - lo);
		const step = pickStep([1, 2, 5, 10, 15, 20, 25, 50], span / 4);
		const min = Math.floor(lo / step) * step;
		const max = Math.ceil(hi / step) * step;
		const ticks: number[] = [];
		for (let t = min; t <= max + 1e-6; t += step) ticks.push(t);
		return { min, max, ticks };
	}

	function formatClock(ts: number, dateOnly = false) {
		const d = new Date(ts);
		if (dateOnly) return `${d.getMonth() + 1}/${d.getDate()}`;
		const hm = `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
		if (!includeDate) return hm;
		return `${d.getMonth() + 1}/${d.getDate()} ${hm}`;
	}

	function formatWatts(n: number, digits = 1): string {
		const body = Math.abs(n).toFixed(digits);
		if (n > 0.005) return `+${body} W`;
		if (n < -0.005) return `-${body} W`;
		return '0 W';
	}

	function formatAxisWatts(n: number): string {
		const body = String(Math.round(n));
		if (n > 0) return `+${body}`;
		if (n < 0) return `${body}`;
		return '0';
	}

	$: timeAxis = (() => {
		const span = Math.max(1, tMax - tMin);
		const target = includeDate ? 4 : 6;
		const stepCandidates = [
			1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1800, 3600, 7200, 10800, 14400, 21600, 43200, 86400, 172800,
			604800,
		].map((s) => s * 1000);
		const step = pickStep(stepCandidates, span / target);
		let first: number;
		if (step >= 86400 * 1000) {
			const d = new Date(tMin);
			d.setHours(0, 0, 0, 0);
			first = d.getTime();
			while (first < tMin) first += step;
		} else {
			first = Math.ceil(tMin / step) * step;
		}
		const ticks: number[] = [];
		for (let t = first; t <= tMax && ticks.length < 12; t += step) ticks.push(t);
		return { ticks, step };
	})();

	function lerpMaybe(a: number | null | undefined, b: number | null | undefined, t: number): number | undefined {
		if (a == null && b == null) return undefined;
		if (a == null) return b ?? undefined;
		if (b == null) return a ?? undefined;
		return a + (b - a) * t;
	}

	function valuesAtTime(targetTs: number): { ts: number; chargePct?: number; watts?: number } | null {
		const bracket = bracketByTime(hoverSamples, targetTs, (s) => s.ts_ms);
		if (!bracket) return null;
		const { left, right } = bracket;
		if (left !== right && targetTs > left.ts_ms && targetTs < right.ts_ms && right.ts_ms - left.ts_ms > GAP_MS) {
			const t = (targetTs - left.ts_ms) / (right.ts_ms - left.ts_ms);
			return {
				ts: targetTs,
				chargePct: lerpMaybe(left.charge_pct, right.charge_pct, t),
			};
		}
		const nearest = Math.abs(right.ts_ms - targetTs) < Math.abs(left.ts_ms - targetTs) ? right : left;
		return {
			ts: nearest.ts_ms,
			chargePct: nearest.charge_pct ?? undefined,
			watts: nearest.watts ?? undefined,
		};
	}

	function onMouseMove(e: MouseEvent) {
		if (!svgEl) return;
		const targetTs = pointerToDomainX(e.clientX, svgEl, svgWidth, padding, tMin, tMax);
		if (targetTs == null) return;
		const sample = valuesAtTime(targetTs);
		if (!sample) {
			hover = null;
			return;
		}
		const chargeY = sample.chargePct != null ? yToPxPct(sample.chargePct) : null;
		const wattY = sample.watts != null ? yToPxWatts(sample.watts) : null;
		const rect = svgEl.getBoundingClientRect();
		const relY = e.clientY - rect.top;
		const scaleY = rect.height / svgHeight;
		let anchor: 'charge' | 'power' = chargeY != null ? 'charge' : 'power';
		let anchorY = chargeY ?? wattY ?? padding.top;
		if (chargeY != null && wattY != null) {
			const chargeDist = Math.abs(chargeY * scaleY - relY);
			const wattDist = Math.abs(wattY * scaleY - relY);
			if (wattDist < chargeDist) {
				anchor = 'power';
				anchorY = wattY;
			} else {
				anchorY = chargeY;
			}
		}
		hover = {
			ts: sample.ts,
			chargePct: sample.chargePct,
			watts: sample.watts,
			anchorY,
			anchor,
		};
	}

	function onMouseLeave() {
		hover = null;
	}

	function startHistoryTimer() {
		if (historyTimer) clearInterval(historyTimer);
		historyTimer = setInterval(fetchHistory, pollSecs * 1000);
	}

	function applyPoll(ms: number) {
		pollSecs = Math.round(ms / 1000);
		if (historyTimer) startHistoryTimer();
	}

	onDestroy(
		followConfig({
			select: (c) => c.battery.poll_ms ?? DEFAULT_POLL_SECS * 1000,
			apply: applyPoll,
		}),
	);

	onMount(async () => {
		try {
			const saved = localStorage.getItem(WINDOW_KEY);
			if (saved && WINDOW_OPTIONS.includes(saved)) windowChoice = saved;
		} catch {}
		await fetchHistory();
		startHistoryTimer();
	});

	async function fetchHistory() {
		try {
			const since = samples.length ? samples[samples.length - 1].ts_ms : undefined;
			const next = (await DefaultService.getBatteryHistory(since)) || [];
			if (since == null) samples = next;
			else if (next.length) samples = samples.concat(next);
			nowMs = Date.now();
			historyError = null;
		} catch (e) {
			historyError = e instanceof Error ? e.message : String(e);
		}
	}

	function saveWindow() {
		try {
			localStorage.setItem(WINDOW_KEY, windowChoice);
		} catch {}
	}

	async function savePoll() {
		try {
			await patch({ battery: { poll_ms: pollSecs * 1000 } });
			startHistoryTimer();
		} catch {}
	}

	onDestroy(() => {
		if (historyTimer) clearInterval(historyTimer);
	});
</script>

<GraphPanel>
	<svelte:fragment slot="top" let:openSettings>
		<div class="flex flex-wrap items-center gap-2 text-xs gap-y-1 pl-[2px]">
			<span class="inline-flex items-center gap-1">
				<span class="w-2.5 h-2.5 rounded-sm" style={`background:${CHARGE_COLOR}`}></span>
				<span class="opacity-80">Charge</span>
				{#if last?.charge_pct != null}
					<span class="tabular-nums font-medium">{last.charge_pct.toFixed(0)}%</span>
				{/if}
			</span>
			<span class="inline-flex items-center gap-1">
				<span class="w-2.5 h-2.5 rounded-sm" style={`background:${POWER_COLOR}`}></span>
				<span class="opacity-80">Power</span>
				{#if last?.watts != null}
					<span class="tabular-nums font-medium">{formatWatts(last.watts)}</span>
				{/if}
			</span>
		</div>
		<div class="flex gap-2">
			<button class="btn btn-xs btn-ghost" on:click={openSettings} aria-label="Open settings">
				<Icon icon="mdi:cog-outline" class="text-base" />
			</button>
		</div>
	</svelte:fragment>

	<svelte:fragment slot="graph">
		<div
			class="relative h-full min-h-0"
			use:measureSize={{ onChange: applyGraphSize }}
		>
			<svg
				bind:this={svgEl}
				class="absolute inset-0 w-full h-full bg-base-100 rounded border border-base-300"
				viewBox={`0 0 ${svgWidth} ${svgHeight}`}
				preserveAspectRatio="none"
				role="img"
				aria-label="Battery charge and power history"
				on:mousemove={onMouseMove}
				on:mouseleave={onMouseLeave}
			>
				<rect width={svgWidth} height={svgHeight} fill="transparent" />
				<defs>
					<clipPath id="battery-plot-clip">
						<rect
							x={padding.left}
							y={padding.top}
							width={svgWidth - padding.left - padding.right}
							height={svgHeight - padding.top - padding.bottom}
						/>
					</clipPath>
				</defs>

				<g stroke="currentColor" class="opacity-30">
					<line
						x1={padding.left}
						y1={yToPxPct(0, svgHeight)}
						x2={svgWidth - padding.right}
						y2={yToPxPct(0, svgHeight)}
						stroke-width="1"
					/>
					<line
						x1={padding.left}
						y1={padding.top}
						x2={padding.left}
						y2={svgHeight - padding.bottom}
						stroke-width="1"
					/>
					<line
						x1={svgWidth - padding.right}
						y1={padding.top}
						x2={svgWidth - padding.right}
						y2={svgHeight - padding.bottom}
						stroke-width="1"
					/>
				</g>

				{#each [0, 20, 40, 60, 80, 100] as d (d)}
					<g>
						<line
							x1={padding.left}
							y1={yToPxPct(d, svgHeight)}
							x2={svgWidth - padding.right}
							y2={yToPxPct(d, svgHeight)}
							stroke="currentColor"
							class="opacity-10"
						/>
						<text x={padding.left - 6} y={yToPxPct(d, svgHeight) + 4} text-anchor="end" class="fill-current opacity-60 text-[10px]"
							>{d}%</text
						>
					</g>
				{/each}

				{#if wattScale.min < 0 && wattScale.max > 0}
					<line
						x1={padding.left}
						y1={yToPxWatts(0, svgHeight)}
						x2={svgWidth - padding.right}
						y2={yToPxWatts(0, svgHeight)}
						stroke="currentColor"
						stroke-width="1"
						class="opacity-30"
					/>
				{/if}

				{#each timeAxis.ticks as t (t)}
					<g>
						<line
							x1={xToPx(t, svgWidth)}
							y1={padding.top}
							x2={xToPx(t, svgWidth)}
							y2={svgHeight - padding.bottom}
							stroke="currentColor"
							class="opacity-10"
						/>
						<text x={xToPx(t, svgWidth)} y={svgHeight - padding.bottom + 16} text-anchor="middle" class="fill-current opacity-60 text-[10px]"
							>{formatClock(t, timeAxis.step >= 86400 * 1000)}</text
						>
					</g>
				{/each}

				<g clip-path="url(#battery-plot-clip)">
					{#each plotSeries as series (series.key)}
						{#each series.segs as seg (seg[0]?.[0])}
							<path d={buildPath(seg, series.yToPx, svgWidth, svgHeight)} fill="none" stroke={series.color} stroke-width="2" />
						{/each}
						{#each series.gaps as [a, b] (a[0])}
							<path
								d={`M${xToPx(a[0], svgWidth)},${series.yToPx(a[1], svgHeight)} L${xToPx(b[0], svgWidth)},${series.yToPx(b[1], svgHeight)}`}
								fill="none"
								stroke={series.color}
								stroke-width="1.5"
								stroke-dasharray="4,4"
								opacity="0.7"
							/>
						{/each}
					{/each}
				</g>

				{#each wattScale.ticks as w (w)}
					<text
						x={svgWidth - padding.right + 6}
						y={yToPxWatts(w, svgHeight) + 4}
						text-anchor="start"
						class="fill-current opacity-60 text-[10px]">{formatAxisWatts(w)}W</text
					>
				{/each}

				{#if hover}
					<line
						x1={xToPx(hover.ts, svgWidth)}
						y1={padding.top}
						x2={xToPx(hover.ts, svgWidth)}
						y2={svgHeight - padding.bottom}
						stroke="currentColor"
						class="opacity-40"
						stroke-dasharray="3,3"
					/>
					<circle
						bind:this={hoverCircleEl}
						cx={xToPx(hover.ts, svgWidth)}
						cy={hover.anchorY}
						r="3.5"
						fill={hover.anchor === 'charge' ? CHARGE_COLOR : POWER_COLOR}
						stroke="white"
						stroke-width="1.5"
					/>
				{/if}
			</svg>

			{#if historyError}
				<div class="absolute inset-0 flex items-center justify-center text-xs text-error pointer-events-none px-4 text-center">
					{historyError}
				</div>
			{:else if samples.length === 0}
				<div class="absolute inset-0 flex items-center justify-center text-xs opacity-50 pointer-events-none">
					Collecting battery history…
				</div>
			{:else if windowed.length === 0}
				<div class="absolute inset-0 flex items-center justify-center text-xs opacity-50 pointer-events-none">
					No samples in this window
				</div>
			{/if}

			<div
				use:tooltip={{
					anchor: () => hoverCircleEl,
					visible: !!hover,
					attachGlobalDismiss: false,
				}}
				class="pointer-events-none whitespace-nowrap bg-base-200 px-2 py-1 rounded border border-base-300 shadow text-xs"
			>
				{#if hover}
					<div class="opacity-60 mb-0.5">{formatClock(hover.ts)}</div>
					{#if hover.chargePct != null}
						<div class="flex items-center gap-2">
							<span class="inline-block w-2.5 h-2.5 rounded-sm" style={`background:${CHARGE_COLOR}`}></span>
							<span class="opacity-80">Charge</span>
							<span class="tabular-nums font-medium">{hover.chargePct.toFixed(1)}%</span>
						</div>
					{/if}
					{#if hover.watts != null}
						<div class="flex items-center gap-2">
							<span class="inline-block w-2.5 h-2.5 rounded-sm" style={`background:${POWER_COLOR}`}></span>
							<span class="opacity-80">Power</span>
							<span class="tabular-nums font-medium">{formatWatts(hover.watts)}</span>
						</div>
					{/if}
				{/if}
			</div>
		</div>
	</svelte:fragment>

	<svelte:fragment slot="settings">
		<div class="flex-1 flex flex-col justify-evenly space-y-2">
			<UiControlCard
				label="History sample interval"
				icon="mdi:timer-outline"
				unit="s"
				min={MIN_POLL_SECS}
				max={MAX_POLL_SECS}
				step={5}
				bind:value={pollSecs}
				on:change={savePoll}
			/>
			<UiControlCard
				label="Window"
				icon="mdi:timeline-clock-outline"
				variant="select"
				options={WINDOW_OPTIONS}
				bind:value={windowChoice}
				on:change={saveWindow}
			/>
		</div>
	</svelte:fragment>
</GraphPanel>

<style>
	.tabular-nums {
		font-variant-numeric: tabular-nums;
	}
</style>
