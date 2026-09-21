import daisyui from 'daisyui';
import { FRAMEWORK_THEMES } from './theme.config.js';

/** @type {import('tailwindcss').Config} */
export default {
	content: ['./index.html', './src/**/*.{svelte,ts,js}'],
	theme: {
		extend: {},
	},
	plugins: [daisyui],
	daisyui: {
		themes: FRAMEWORK_THEMES,
	},
};
