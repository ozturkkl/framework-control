import './app.css';
import App from './App.svelte';
import { OpenAPI } from './api';

OpenAPI.BASE = import.meta.env?.VITE_API_BASE || 'http://127.0.0.1:8090';

const app = new App({ target: document.getElementById('app')! });
export default app;


