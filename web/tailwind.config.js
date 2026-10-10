import typography from '@tailwindcss/typography';
import daisyui from 'daisyui';
import { FRAMEWORK_THEMES } from './theme.config.js';

/** @type {import('tailwindcss').Config} */
export default {
	content: ['./index.html', './src/**/*.{svelte,ts,js}'],
	// ANNOUNCEMENT.md is fetched at runtime and never scanned, so it can only use classes listed here or already used in src.
	safelist: [
		{ pattern: /^(m|mx|my|mt|mb|ml|mr|p|px|py|pt|pb|pl|pr)-(0|0\.5|1|1\.5|2|3|4|6|8|auto)$/ },
		{ pattern: /^(gap|gap-x|gap-y|space-x|space-y)-(0|1|2|3|4|6|8)$/ },
		{ pattern: /^(block|inline-block|inline|flex|inline-flex|grid|hidden)$/ },
		{ pattern: /^flex-(row|col|wrap|nowrap|1|auto|none)$/ },
		{ pattern: /^(grow|grow-0|shrink|shrink-0)$/ },
		{ pattern: /^items-(start|center|end|baseline|stretch)$/ },
		{ pattern: /^justify-(start|center|end|between|around|evenly)$/ },
		{ pattern: /^self-(start|center|end)$/ },
		{ pattern: /^grid-cols-(1|2|3|4)$/ },
		{ pattern: /^text-(left|center|right)$/ },
		{ pattern: /^(w-full|w-auto|max-w-full)$/ },
		{ pattern: /^rounded(-lg|-xl)?$/ },
	],
	theme: {
		extend: {},
	},
	plugins: [typography, daisyui],
	daisyui: {
		themes: FRAMEWORK_THEMES,
	},
};
