import { describe, expect, it } from 'vitest';
import { lastChargeStartMs, splitByWattSign, type Pt } from './batteryHistory';

describe('lastChargeStartMs', () => {
	it('returns when AC was most recently plugged in', () => {
		const samples = [
			{ ts_ms: 1, ac_present: true },
			{ ts_ms: 2, ac_present: false },
			{ ts_ms: 3, ac_present: true },
			{ ts_ms: 4, ac_present: true },
			{ ts_ms: 5, ac_present: false },
		];
		expect(lastChargeStartMs(samples)).toBe(3);
		expect(lastChargeStartMs([{ ts_ms: 1, ac_present: false }, { ts_ms: 2 }])).toBeNull();
	});
});

describe('splitByWattSign', () => {
	it('ends and starts the fills at the interpolated 0W crossing', () => {
		const seg: Pt[] = [
			[0, 10],
			[10, -10],
		];
		expect(splitByWattSign([seg], 1)).toEqual([
			[
				[0, 10],
				[5, 0],
			],
		]);
		expect(splitByWattSign([seg], -1)).toEqual([
			[
				[5, 0],
				[10, -10],
			],
		]);
	});

	it('shares an exact 0W sample between both sides', () => {
		const seg: Pt[] = [
			[0, 5],
			[10, 0],
			[20, -5],
		];
		expect(splitByWattSign([seg], 1)).toEqual([
			[
				[0, 5],
				[10, 0],
			],
		]);
		expect(splitByWattSign([seg], -1)).toEqual([
			[
				[10, 0],
				[20, -5],
			],
		]);
	});
});
