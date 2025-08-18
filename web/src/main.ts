import './app.css';
import App from './App.svelte';
import { OpenAPI } from './api';

OpenAPI.BASE = import.meta.env?.VITE_API_BASE || 'http://127.0.0.1:8090';
const rawToken = (import.meta.env?.VITE_CONTROL_TOKEN ?? '').toString().trim();
OpenAPI.TOKEN = rawToken.length > 0 ? rawToken : undefined;


const app = new App({ target: document.getElementById('app')! });
export default app;


