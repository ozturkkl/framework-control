import type { BatterySample } from '../api';

export type Pt = [number, number];

/** Start of the most recent run of AC-present samples ("Since last charge"). */
export function lastChargeStartMs(list: BatterySample[]): number | null {
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

function zeroCrossing(a: Pt, b: Pt): Pt {
	const dy = b[1] - a[1];
	const t = dy === 0 ? 0 : (0 - a[1]) / dy;
	return [a[0] + t * (b[0] - a[0]), 0];
}

function wattSign(y: number): 1 | -1 | 0 {
	if (y > 0) return 1;
	if (y < 0) return -1;
	return 0;
}

/** Keep runs on one side of 0W, inserting interpolated crossings so fills meet the baseline. */
export function splitByWattSign(segs: Pt[][], sign: 1 | -1): Pt[][] {
	const out: Pt[][] = [];
	for (const points of segs) {
		let cur: Pt[] = [];
		const flush = () => {
			if (cur.length >= 2 && cur.some((p) => p[1] !== 0)) out.push(cur);
			cur = [];
		};
		for (let i = 0; i < points.length; i++) {
			const p = points[i];
			const s = wattSign(p[1]);
			if (i > 0) {
				const prev = points[i - 1];
				const ps = wattSign(prev[1]);
				if (ps !== 0 && s !== 0 && ps !== s) {
					const z = zeroCrossing(prev, p);
					if (ps === sign) {
						cur.push(z);
						flush();
					} else {
						cur = [z];
					}
				}
			}
			if (s === sign || s === 0) cur.push(p);
			else flush();
		}
		flush();
	}
	return out;
}
