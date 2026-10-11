import { describe, expect, it } from 'vitest';
import { bracketByTime, findNearestByTime, wattsScale, wattYToPx } from './plot';

const samples = [{ t: 0 }, { t: 10 }, { t: 20 }];
const getTime = (s: { t: number }) => s.t;

describe('findNearestByTime', () => {
	it('returns the closest sample, clamping to the ends', () => {
		expect(findNearestByTime(samples, 4, getTime)).toBe(samples[0]);
		expect(findNearestByTime(samples, 6, getTime)).toBe(samples[1]);
		expect(findNearestByTime(samples, -5, getTime)).toBe(samples[0]);
		expect(findNearestByTime(samples, 99, getTime)).toBe(samples[2]);
	});
});

describe('bracketByTime', () => {
	it('returns the samples on either side of the target', () => {
		expect(bracketByTime(samples, 15, getTime)).toEqual({ left: samples[1], right: samples[2] });
	});
});

describe('wattsScale', () => {
	const datasets = [[], [5, -3], [150], [-90], [100, -80]];

	it('covers the data and keeps 0W at 40% of the axis height', () => {
		for (const values of datasets) {
			const { min, max } = wattsScale(values);
			expect(max).toBeGreaterThanOrEqual(Math.max(0, ...values));
			expect(min).toBeLessThanOrEqual(Math.min(0, ...values));
			expect(-min / (max - min)).toBeCloseTo(0.4);
		}
	});

	it('maps the scale bounds to the plot edges', () => {
		const padding = { top: 10, bottom: 30 };
		const scale = wattsScale([60, -40]);
		expect(wattYToPx(scale.max, scale, 200, padding)).toBeCloseTo(padding.top);
		expect(wattYToPx(scale.min, scale, 200, padding)).toBeCloseTo(200 - padding.bottom);
	});
});
