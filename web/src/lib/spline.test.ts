import { describe, expect, it } from 'vitest';
import { cubicSplineInterpolate } from './spline';

describe('cubicSplineInterpolate', () => {
	it('passes through every control point regardless of input order', () => {
		const points: [number, number][] = [
			[3000, 60],
			[0, 0],
			[5000, 100],
			[1200, 20],
			[4200, 80],
		];
		for (const [x, y] of points) {
			expect(cubicSplineInterpolate(points, x)).toBeCloseTo(y);
		}
	});

	it('stays finite when control points share an x value', () => {
		// Calibration where the fan is still stopped at 20% duty: inverted to rpm -> duty, x=0 repeats.
		const points: [number, number][] = [
			[0, 0],
			[0, 20],
			[1500, 40],
			[3000, 60],
			[4500, 80],
			[6000, 100],
		];
		for (const rpm of [0, 750, 1500, 2200, 6000]) {
			expect(Number.isFinite(cubicSplineInterpolate(points, rpm))).toBe(true);
		}
		expect(cubicSplineInterpolate(points, 3000)).toBeCloseTo(60);
	});
});
