import { writable } from 'svelte/store';
import { DefaultService, OpenAPI, type Config, type PartialConfig } from '../api';

const CLIENT_ID = crypto.randomUUID();

type ConfigState = {
	config: Config | null;
	loaded: boolean;
	error: string | null;
	ownWrite: boolean;
};

export const configStore = writable<ConfigState>({
	config: null,
	loaded: false,
	error: null,
	ownWrite: false,
});

export function followConfig<T>({
	select,
	apply,
}: {
	select: (config: Config) => T | undefined;
	apply: (slice: T) => void;
}): () => void {
	let lastJson = '';
	return configStore.subscribe((s) => {
		const slice = s.config ? select(s.config) : undefined;
		if (slice === undefined) return;
		const json = JSON.stringify(slice);
		if (json === lastJson) return;
		const seed = lastJson === '';
		lastJson = json;
		if (s.ownWrite && !seed) return;
		apply(slice);
	});
}

/** Bumped on every set so an in-flight GET cannot overwrite a newer snapshot. */
let eventEpoch = 0;

function setStore(cfg: Config, ownWrite: boolean) {
	eventEpoch += 1;
	configStore.set({ config: cfg, loaded: true, error: null, ownWrite });
}

let loadInFlight: Promise<void> | null = null;

async function loadConfigFromServer(): Promise<void> {
	if (loadInFlight) return loadInFlight;
	const epoch = eventEpoch;
	loadInFlight = (async () => {
		try {
			const cfg = await DefaultService.getConfig();
			if (epoch !== eventEpoch) return;
			setStore(cfg, false);
		} catch (e) {
			configStore.update((s) => ({
				...s,
				error: e instanceof Error ? e.message : String(e),
			}));
		}
	})().finally(() => {
		loadInFlight = null;
	});
	return loadInFlight;
}

export async function patch(partial: PartialConfig): Promise<void> {
	const cfg = await DefaultService.setConfig(partial, CLIENT_ID);
	setStore(cfg, true);
}

let eventSource: EventSource | null = null;

export function startConfigEvents(): void {
	if (eventSource) return;
	eventSource = new EventSource(`${OpenAPI.BASE}/config/events`);
	loadConfigFromServer();
	eventSource.onopen = () => {
		loadConfigFromServer();
	};
	eventSource.onmessage = (ev) => {
		try {
			const data = JSON.parse(ev.data) as { client_id?: string | null; config?: Config };
			if (data.client_id === CLIENT_ID) return;
			if (data.config) setStore(data.config, false);
		} catch {}
	};
}
