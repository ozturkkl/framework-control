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

export function findNearestByTime<T>(
	items: T[],
	target: number,
	getTime: (item: T) => number,
): T | null {
	const bracket = bracketByTime(items, target, getTime);
	if (!bracket) return null;
	const { left, right } = bracket;
	return Math.abs(getTime(right) - target) < Math.abs(getTime(left) - target) ? right : left;
}
