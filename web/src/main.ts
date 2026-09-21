import './app.css';
import App from './App.svelte';
import { OpenAPI } from './api';
import { startConfigEvents, followConfig } from './lib/config';
import { themes } from './lib/themes';

// Derive API base from current origin unless explicitly overridden
OpenAPI.BASE = (import.meta.env?.VITE_API_BASE as string | undefined) || `${window.location.origin}/api`;

// Apply saved theme early so initial render uses it
let savedTheme: string | null = null;
try {
	savedTheme = localStorage.getItem('fc_theme');
} catch {}
themes.initialize(savedTheme);

startConfigEvents();
followConfig({
	select: (c) => c.ui?.theme,
	apply: (theme) => {
		const preference = themes.apply(theme);
		if (theme !== preference) return;
		try {
			localStorage.setItem('fc_theme', preference);
		} catch {}
	},
});

const app = new App({ target: document.getElementById('app')! });
export default app;
