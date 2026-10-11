import { describe, expect, it } from 'vitest';
import { formatEtaMinutes, minutesToChargeTarget, minutesToDrainTarget } from './batteryEta';

const pack = { last_full_charge_capacity_mah: 5000, present_rate_ma: 1500 };

describe('minutesToChargeTarget', () => {
	it('estimates the time to fill the gap up to the charge limit', () => {
		expect(minutesToChargeTarget({ ...pack, charging: true, percentage: 50 }, 80)).toBe(60);
	});
});

describe('minutesToDrainTarget', () => {
	it('estimates the time to drain down to the limit when above it', () => {
		expect(minutesToDrainTarget({ ...pack, present_rate_ma: 1000, percentage: 90 }, 80)).toBe(30);
	});

	it('gives no estimate at or below the limit', () => {
		expect(minutesToDrainTarget({ ...pack, percentage: 80 }, 80)).toBeUndefined();
	});
});

describe('formatEtaMinutes', () => {
	it('shows at most two units', () => {
		expect(formatEtaMinutes(45)).toBe('45m');
		expect(formatEtaMinutes(90)).toBe('1h 30m');
		expect(formatEtaMinutes(59.6)).toBe('1h');
		expect(formatEtaMinutes(1500)).toBe('1d 1h');
		expect(formatEtaMinutes(2880)).toBe('2d');
	});

	it('shows a dash for zero or non-finite durations', () => {
		expect(formatEtaMinutes(0)).toBe('—');
		expect(formatEtaMinutes(Infinity)).toBe('—');
	});
});
