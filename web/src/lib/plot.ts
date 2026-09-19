export type PlotPadding = {
	left: number;
	right: number;
	top: number;
	bottom: number;
};

/** CSS-pixel mouse X → viewBox X (same mapping as a 1:1 viewBox). */
export function pointerViewX(clientX: number, svg: SVGSVGElement, svgWidth: number): number {
	const rect = svg.getBoundingClientRect();
	const fracX = Math.max(0, Math.min(1, (clientX - rect.left) / Math.max(1, rect.width)));
	return fracX * svgWidth;
}

/** ViewBox X → domain X. Positions in the axis gutters extrapolate past the domain. */
export function viewXToDomain(
	xView: number,
	svgWidth: number,
	padding: Pick<PlotPadding, 'left' | 'right'>,
	xMin: number,
	xMax: number,
): number | null {
	const w = svgWidth - padding.left - padding.right;
	if (w <= 0) return null;
	if (xMax === xMin) return xMin;
	return xMin + ((xView - padding.left) / w) * (xMax - xMin);
}

export function pointerToDomainX(
	clientX: number,
	svg: SVGSVGElement,
	svgWidth: number,
	padding: Pick<PlotPadding, 'left' | 'right'>,
	xMin: number,
	xMax: number,
): number | null {
	return viewXToDomain(pointerViewX(clientX, svg, svgWidth), svgWidth, padding, xMin, xMax);
}

export function bracketByTime<T>(
	items: T[],
	target: number,
	getTime: (item: T) => number,
): { left: T; right: T } | null {
	if (!items.length) return null;
	let lo = 0;
	let hi = items.length - 1;
	while (lo < hi) {
		const mid = Math.floor((lo + hi) / 2);
		if (getTime(items[mid]) < target) lo = mid + 1;
		else hi = mid;
	}
	const right = items[lo];
	const left = lo > 0 ? items[lo - 1] : right;
	return { left, right };
}

export function findNearestByTime<T>(items: T[], target: number, getTime: (item: T) => number): T | null {
	const bracket = bracketByTime(items, target, getTime);
	if (!bracket) return null;
	const { left, right } = bracket;
	return Math.abs(getTime(right) - target) < Math.abs(getTime(left) - target) ? right : left;
}

export type WattScale = { min: number; max: number; ticks: number[] };

const WATT_EXPECTED = 20;
const WATT_PAD_BOTTOM = 10;
const WATT_PAD_TOP = 30;
const WATT_ZERO_CHARGE_PCT = 40;

function pickWattStep(approx: number): number {
	const candidates = [1, 2, 5, 10, 15, 20, 25, 50];
	for (const c of candidates) {
		if (c >= approx) return c;
	}
	return candidates[candidates.length - 1];
}

/** Bipolar W scale: 0W stays put, ±20 expected, then +30W top / −10W bottom padding. */
export function wattsScale(values: number[]): WattScale {
	let dataMin = 0;
	let dataMax = 0;
	for (const v of values) {
		if (Number.isFinite(v)) {
			dataMin = Math.min(dataMin, v);
			dataMax = Math.max(dataMax, v);
		}
	}
	const min = Math.min(-WATT_EXPECTED, dataMin) - WATT_PAD_BOTTOM;
	const max = Math.max(WATT_EXPECTED, dataMax) + WATT_PAD_TOP;
	const step = pickWattStep((max - min) / 4);
	const ticks = [0];
	for (let t = step; t <= max + 1e-6; t += step) ticks.push(t);
	for (let t = -step; t >= min - 1e-6; t -= step) ticks.push(t);
	ticks.sort((a, b) => a - b);
	return { min, max, ticks };
}

export function wattYToPx(
	watts: number,
	scale: Pick<WattScale, 'min' | 'max'>,
	height: number,
	padding: Pick<PlotPadding, 'top' | 'bottom'>,
): number {
	const h = height - padding.top - padding.bottom;
	const zeroY = padding.top + (1 - WATT_ZERO_CHARGE_PCT / 100) * h;
	if (watts >= 0) {
		if (scale.max === 0) return zeroY;
		return zeroY + (watts / scale.max) * (padding.top - zeroY);
	}
	if (scale.min === 0) return zeroY;
	return zeroY + (watts / scale.min) * (height - padding.bottom - zeroY);
}
