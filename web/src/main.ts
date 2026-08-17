import './app.css';
import App from './App.svelte';
import { OpenAPI } from './api';
import { startConfigEvents, followConfig } from './lib/config';

// Derive API base from current origin unless explicitly overridden
OpenAPI.BASE = (import.meta.env?.VITE_API_BASE as string | undefined) || `${window.location.origin}/api`;

// Apply saved theme early so initial render uses it
try {
	const savedTheme = localStorage.getItem('fc_theme');
	if (savedTheme) {
		document.documentElement.setAttribute('data-theme', savedTheme);
	}
} catch {}

startConfigEvents();
followConfig({
	select: (c) => c.ui?.theme,
	apply: (theme) => {
		document.documentElement.setAttribute('data-theme', theme);
		try {
			localStorage.setItem('fc_theme', theme);
		} catch {}
	},
});

const app = new App({ target: document.getElementById('app')! });
export default app;
