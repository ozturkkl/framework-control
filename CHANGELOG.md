## Unreleased

- UX: Migrate all tooltips/popups to the new minimal `web/src/lib/tooltip.ts`; remove legacy `web/src/lib/tooltipPortal.ts` and DaisyUI `.tooltip` usage.
- UX: Add minimal element-based tooltip action (`web/src/lib/tooltip.ts`) with `visible` flag control, fixed portal positioning, auto vertical flip, and viewport clamping. Deprecates and removes DaisyUI-oriented `tooltipPortal.ts`.
- UI: Simplify responsive layout. Wide screens now use two columns (healthy) and three columns (pre‑healthy) with auto height using CSS columns; cards no longer equalize heights. Removed flip and header animations so cards snap into place. Removed animations in `DeviceHeader` (image crossfade, width transition, pulsing dot). Moved fan mode selector into `FanControl`.
- UI: Fan mode selector is now overlaid in the Fan Control panel header (like Power Control).
- UI: Standardize chart card padding/spacing between Telemetry and Fan Control for better side‑by‑side alignment.
- UI: Panels now have consistent header-to-content spacing. Header adds margin-bottom; first child content’s top margin/padding is reset for uniformity.
- Layout: Replace CSS columns with a simple `flex-wrap` layout in `App.svelte` to preserve source order and allow dynamic card heights (2 cols when healthy, 3 cols pre‑healthy).
- Layout: Unhealthy state now equalizes panel heights per row for a tidier look.
- Rename: "Telemetry" panel label changed to "Sensors".
- Rename: Frontend component `Telemetry.svelte` renamed to `Sensors.svelte`.
- UI: Introduce `GraphPanel.svelte` wrapper to DRY chart card layout (shared padding, sticky settings header, matched settings height). `Sensors.svelte` and Fan Control (Curve) now consume this wrapper.
- UI: Sensors and Fan Control settings keep the same height as their graph panels; settings content centers when shorter and scrolls vertically when taller to avoid layout shifts.
- Fix: Settings height measurement now includes the header row to avoid small jumps when header contents differ between graph and settings views.
- UX: Settings header (with Back button) is sticky while scrolling so the Back action remains visible.
- Fix: Sticky settings header uses a solid background and no negative margins; horizontal overflow scrollbars removed.
- Reliability: After `framework_tool` becomes unavailable and then reappears, the fan task now forces a reapply (clears cached `last_duty` and re-anchors curve), so Curve/Manual modes engage without needing a config change.
- Auto update after install will no longer make desktop shortcuts reappear
- Telemetry: Added temperature sensor values as a graph. Moved the current TDP and thermal limit display to the Power Panel
- Fix: `MultiSelect` now uses per-instance unique input IDs to prevent selection bleeding between Fan Control and Sensors.
- UX: `MultiSelect` dropdown auto-flips left/right alignment to avoid overflowing off-screen when near viewport edges.
- UX: Tooltips now render via a small portal-based action (`tooltipPortal`) instead of DaisyUI pseudo-elements. This removes modal clipping and prevents transient horizontal scrollbars. Removed the old `tooltipClamp` helper and its CSS.
- UX: Sensors graph adds a hover crosshair and nearest-point tooltip showing sensor name, value, and relative time.
- UX: Fan Control curve editor shows a compact tooltip next to the active point (also visible while dragging) with a colored indicator and bold values (e.g., `30°C · 30%`), without brackets and with overflow-aware positioning.
- UX: Fan Control focused point can be moved with arrow keys (Shift=±5, Ctrl=±10; Home/End to min/max temperature).

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
