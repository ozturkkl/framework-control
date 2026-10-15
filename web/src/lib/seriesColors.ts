// Stable hue order backed by the active theme's sensor palette.
export const SERIES_PALETTE = [
	'var(--sensor-green)',
	'var(--sensor-blue)',
	'var(--sensor-red)',
	'var(--sensor-purple)',
	'var(--sensor-amber)',
	'var(--sensor-emerald)',
	'var(--sensor-orange)',
	'var(--sensor-cyan)',
	'var(--sensor-yellow)',
	'var(--sensor-violet)',
	'var(--sensor-pink)',
	'var(--sensor-teal)',
];

export function hashColor(name: string): string {
	let h = 0 >>> 0;
	for (let i = 0; i < name.length; i++) {
		h = (h * 31 + name.charCodeAt(i)) >>> 0;
	}
	return SERIES_PALETTE[h % SERIES_PALETTE.length];
}
