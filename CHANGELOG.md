## Unreleased

- Reliability: After `framework_tool` becomes unavailable and then reappears, the fan task now forces a reapply (clears cached `last_duty` and re-anchors curve), so Curve/Manual modes engage without needing a config change.
- Auto update after install will no longer make desktop shortcuts reappear
- Telemetry: Added temperature sensor values as a graph. Moved the current TDP and thermal limit display to the Power Panel

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
