## Unreleased

 - Web: Theme selector (all DaisyUI themes) in Settings. Applies instantly, persists to backend config, and loads on startup.
 - Web: Auto‑reload the page after installing an update from Settings to load new embedded UI assets.
 - UI: Tooltip action now dismisses on outside click and Escape by default (opt‑out with `attachGlobalDismiss: false`), and emits a `dismiss` event for call‑sites to sync visibility if needed.
- Service: Merged `GET /api/battery` into `GET /api/power`. `/api/power` now includes battery stats and charge limit min/max. Removed `GET /api/battery`.
 - Web: Battery panel now reads from `/api/power`; no functional change to write endpoints.
- Breaking: Removed `ac_present` from `/api/power` root. Use `resp.battery.ac_present` instead.
- Service: Power parsing now runs `framework_tool --power -vv` and exposes additional dynamic fields: `charger_voltage_mv`, `charger_current_ma`, `charge_input_current_ma`, `soc_pct`, `design_capacity_mah`, and `design_voltage_mv`. Derived values (Wh, C-rate) are no longer computed by the service.
- UI: Battery panel info bar shows live charge/discharge current, computed C‑rate (client-side), pack voltage, SoC, and max charge limit to help verify settings at a glance.
- Service: Battery settings moved to config + background task. Added `config.battery` with:
  - `charge_limit_max_pct { enabled, value }`
  - `charge_rate_c { enabled, value }`
  - optional `charge_rate_soc_threshold_pct`
  The battery task applies on config change and every 30 minutes.
- Breaking: Removed `POST /api/battery/charge-limit` and `POST /api/battery/rate-limit`. Use `POST /api/config` instead.
- UI: Battery panel now reads/writes via config like Power. Layout now mirrors the Power panel with a separate header card and two slider cards with a gap between them; SoC threshold still inline with presets. When disabled, Rate applies 1.0C; Charge Limit disables by restoring 100%.
- UI: `UiSlider` now supports a header `trailing` slot (for chips/menus). Battery Rate uses it to show a subtle SoC threshold chip with an anchored popover (presets + number, auto‑apply, close on Escape/outside click).
- UI: Battery Rate slider: added Enabled toggle; header shows the raw value. When disabled, applies 1.0 C to effectively remove limiting in most cases.
 - UI/Service: Clearing the Battery rate SoC threshold now correctly removes it from `config.battery.charge_rate_soc_threshold_pct`.
 - UI: Battery SoC input now only applies changes on blur or Enter, preventing premature application while typing.
 - UI: Power and Battery info bars now wrap by icon+text groups on narrow widths to avoid awkward mid-text wrapping while still showing all values.
 - UI: Battery panel header now shows a compact battery health summary (capacity vs design and cycle count), and charger requested/available power has moved into the Power panel header next to AC/Battery status.

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
