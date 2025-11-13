## Unreleased

 - Web: Theme selector (all DaisyUI themes) in Settings. Applies instantly, persists to backend config, and loads on startup.
 - Web: Auto‑reload the page after installing an update from Settings to load new embedded UI assets.

## 0.4.1 - 2025-11-10

- Sensors: New panel with live graphs and history; moved current TDP and thermal limit to the Power panel.
- Telemetry: Background sampling with configurable poll/retention and a new history API (`/api/thermal/history`).
- Tooltips: Replaced DaisyUI tooltips with a lightweight portal-based action; fixes clipping and stray scrollbars.
- Fan Control: Header mode selector overlay, compact point tooltip + keyboard control, and reliable reapply after tool reconnects.
- Layout/UI: Switched to flex-wrap layout; unified panel/header spacing; standardized chart padding; removed header/flip animations; introduced `web/src/components/GraphPanel.svelte`.
- MultiSelect: Unique input IDs; dropdown auto-aligns left/right; no invisible overlay after closing.
- Windows installer: Prevented desktop/start shortcuts from reappearing on auto-update.
- Misc: Minor spacing tweaks in `Panel` and `UiSlider`.

## 0.4.0 - 2025-10-12

### Backend: Power management, CLI, and reliability

- Add AC/Battery power profiles with per-setting enabled flags; task auto-selects by AC presence and applies only enabled values.
- Implement conservative TDP reapply (quiet window + cooldown, tolerance) using `ryzenadj --info` for improved stability.
- Add minimal 2s TTL cache to reduce repeated CLI calls; clear cache and propagate errors on failures.
- Refactor CLI wrappers under `service/src/cli/` (e.g., `framework_tool.rs`, `ryzen_adj.rs`) for future tool integrations.

### Frontend Web UI: Telemetry, power controls, shortcuts

- Add `Telemetry.svelte`: compact live card showing TDP (W), Thermal (°C), Battery % + charging state via `/api/power`.
- Power panel: AC/Battery tabs, per-setting Enabled flags, AMD-only gating on Intel (via `/api/system`), battery-range hinting.
- Shortcut install: streamlined flow assumes success if API returns; UI reflects created state.

### Shared utilities and developer experience

- Introduce `service/src/utils/` modules: `github`, `wget`, `download`, `zip_extract`, `fs`, `global_cache`.
- Typed, parsed responses across routes to align with generated OpenAPI models.

### Installation and packaging

- Validate `framework_tool` after WinGet; fall back to direct download if resolution fails or tool is missing.
- On-demand RyzenAdj install endpoint with clear errors if installation is blocked (e.g., AV).
- MSI build/docs updated for new env variables and features.

### Breaking changes

- `/api/power`, `/api/thermal`, and `/api/versions` now return parsed, typed JSON objects.
- Frontend policy: always use the generated OpenAPI client (`DefaultService`, `OpenAPI`); do not use `fetch` directly.

### Documentation and versioning

- Update `PROJECT_SUMMARY.md` and `README.md` for new endpoints, UI components, CLI wrappers, and OpenAPI usage.
- Bump service version to `0.4.0`; update dependencies as needed.

### Other improvements

- Fan curve: standardize on `sensors: string[]`. The service uses the max temperature across selected sensors. Frontend shows dynamic sensor list (from `/api/thermal`) and always saves an array (single selection uses a one-element array).
- Windows shortcuts: Brave app-mode support with `.url` fallback; improved detection/status.
- Improved detection and state reporting for missing `framework_tool`.
- Miscellaneous fixes and UI polish.

## 0.3.3 - 2025-09-01

- PWA offline support with passive caching
- Auto-update service (6h checks) + MSI install flow (Windows)
- Windows shortcuts: Start Menu + Desktop
- Embedded static UI in backend; global CORS
- New Settings modal: check/apply updates, auto-install toggle, pause
- Shortcut installer UI
- MSI build + icon generation pipeline
- New env vars: port, update repo, API base, token
- Dependency updates (rust-embed, poem, reqwest, vite-plugin-pwa, etc.)

## 0.1.0 - 2025-09-01

- Initial tracked changelog. Use Unreleased above for next changes.
