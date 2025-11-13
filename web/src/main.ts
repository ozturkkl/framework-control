import "./app.css";
import App from "./App.svelte";
import { OpenAPI } from "./api";
import { DefaultService } from "./api";

// Derive API base from current origin unless explicitly overridden
OpenAPI.BASE =
  (import.meta.env?.VITE_API_BASE as string | undefined) ||
  `${window.location.origin}/api`;
const rawToken = (import.meta.env?.VITE_CONTROL_TOKEN ?? "").toString().trim();
OpenAPI.TOKEN = rawToken.length > 0 ? rawToken : undefined;

// Apply saved theme early so initial render uses it
try {
  const savedTheme = localStorage.getItem("fc_theme");
  if (savedTheme) {
    document.documentElement.setAttribute("data-theme", savedTheme);
  }
} catch {}

// Also query backend config for persisted theme and override if present
try {
  const cfg = await DefaultService.getConfig();
  const backendTheme = cfg?.ui?.theme;
  if (backendTheme) {
    document.documentElement.setAttribute("data-theme", backendTheme);
    localStorage.setItem("fc_theme", backendTheme);
  }
} catch {}

const app = new App({ target: document.getElementById("app")! });
export default app;
