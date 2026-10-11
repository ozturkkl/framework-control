import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { deepMerge, throttleDebounce } from './utils';

describe('deepMerge', () => {
	it('keeps target values where the source is null when noMergeNulls is set', () => {
		const defaults: { ac: { tdp_watts: { enabled: boolean; value: number | null } } } = {
			ac: { tdp_watts: { enabled: false, value: 75 } },
		};
		const merged = deepMerge(defaults, { ac: { tdp_watts: { enabled: true, value: null } } }, true);
		expect(merged).toEqual({ ac: { tdp_watts: { enabled: true, value: 75 } } });
	});

	it('copies nested objects that are missing from the target', () => {
		const target: { ac?: { governor?: { value: string } } } = {};
		expect(deepMerge(target, { ac: { governor: { value: 'performance' } } })).toEqual({
			ac: { governor: { value: 'performance' } },
		});
	});
});

describe('throttleDebounce', () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it('in trailing-only mode, calls once after the window with the latest arguments', () => {
		const fn = vi.fn();
		const throttled = throttleDebounce(fn, 200, false, true);
		throttled(1);
		throttled(2);
		throttled(3);
		expect(fn).not.toHaveBeenCalled();
		vi.advanceTimersByTime(200);
		expect(fn).toHaveBeenCalledTimes(1);
		expect(fn).toHaveBeenCalledWith(3);
	});
});
