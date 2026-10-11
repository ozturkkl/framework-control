import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Config } from '../api';
import { configStore, followConfig } from './config';

// src/api is generated and absent in CI; followConfig never calls it.
vi.mock('../api', () => ({ DefaultService: {}, OpenAPI: { BASE: '' } }));

function setConfig(pollMs: number, ownWrite: boolean) {
	const config = { telemetry: { poll_ms: pollMs } } as Config;
	configStore.set({ config, loaded: true, error: null, ownWrite });
}

describe('followConfig', () => {
	beforeEach(() => {
		configStore.set({ config: null, loaded: false, error: null, ownWrite: false });
	});

	it('applies external changes and skips unchanged slices', () => {
		const apply = vi.fn();
		const stop = followConfig({ select: (c) => c.telemetry.poll_ms, apply });
		setConfig(1000, false);
		setConfig(1000, false);
		setConfig(2000, false);
		stop();
		expect(apply.mock.calls).toEqual([[1000], [2000]]);
	});

	it('seeds from an own write but ignores own-write echoes afterwards', () => {
		const apply = vi.fn();
		const stop = followConfig({ select: (c) => c.telemetry.poll_ms, apply });
		setConfig(1000, true);
		setConfig(2000, true);
		setConfig(3000, false);
		stop();
		expect(apply.mock.calls).toEqual([[1000], [3000]]);
	});
});
