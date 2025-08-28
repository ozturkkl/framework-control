import './app.css';
import App from './App.svelte';
import { OpenAPI } from './api';

// Derive API base from current origin unless explicitly overridden
OpenAPI.BASE = (import.meta.env?.VITE_API_BASE as string | undefined) || `${window.location.origin}/api`;
const rawToken = (import.meta.env?.VITE_CONTROL_TOKEN ?? '').toString().trim();
OpenAPI.TOKEN = rawToken.length > 0 ? rawToken : undefined;

const app = new App({ target: document.getElementById('app')! });
export default app;

// PWA registration (auto update, no prompt)
if (typeof window !== 'undefined') {
  // @ts-ignore: virtual module resolved by VitePWA at build time
  import('virtual:pwa-register')
    .then((mod: { registerSW: (opts: { immediate?: boolean }) => () => void }) => {
      mod.registerSW({ immediate: true });
    })
    .catch(() => {});
}

