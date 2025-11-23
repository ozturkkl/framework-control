## Framework Control – Boilerplate Summary

This document captures the current state, architecture, and key technical details of the Framework Control app so new features can be scoped quickly. Share or update this when implementing changes.

### Purpose

Local Windows service + Svelte web UI to monitor telemetry and control core platform features (fans, power, charging). Uses the official `framework_tool` CLI for EC interactions. Default local API: `http://127.0.0.1:8090`.

### High-Level Architecture

- Backend service: Rust (Tokio + Poem + poem-openapi) in `framework-control/service`
- Frontend web UI: Svelte + Vite in `framework-control/web`
- Packaging: WiX MSI for Windows (service registration, assets)
- Security: loopback-only API; write operations require bearer token

### Backend Service (Rust)

- Entry: `service/src/main.rs` (@main.rs)
  - Loads config and environment (`FRAMEWORK_CONTROL_PORT`, `FRAMEWORK_CONTROL_ALLOWED_ORIGINS`, `FRAMEWORK_CONTROL_TOKEN`)
  - Serves static UI, builds OpenAPI, mounts routes, initializes optional CLI integration
- Routes: `service/src/routes.rs` (@routes.rs)
  - Endpoints (under `/api`):
    - `GET /health`: health + version + `cli_present`
    - `GET /power`: parsed power report with battery info and charge limit min/max
    - `GET /thermal`: parsed thermal report (temps map + fan RPMs)
    - `GET /thermal/history`: recent telemetry samples collected by the service (trimmed by configured retention)
    - `GET /versions`: parsed versions (mainboard_type, uefi_version, etc.)
    - `GET /config`: return persisted config
    - `POST /config`: update config (requires `Authorization: Bearer <token>`)
    - `GET /system`: basic system info (CPU, memory, OS, dGPU guess)
    - `GET /shortcuts/status`: Start menu/Desktop shortcut existence
    - `POST /shortcuts/create`: create app-mode browser shortcuts (auth required)
- `POST /ryzenadj/install`: download/install RyzenAdj on demand (auth required)
- `POST /ryzenadj/uninstall`: remove downloaded RyzenAdj artifacts and clear state (auth required)
  - `GET /update/check`: check for latest version from update feed (see env below)
  - `POST /update/apply`: install the update (auth required)
- Helpers: GPU detection via PowerShell on Windows
- Other key files (condensed):
  - `service/src/config.rs`: load/save config JSON at `C:\ProgramData\FrameworkControl\config.json`
  - `service/src/types.rs`: API and config types; includes power AC/Battery profiles, battery settings (`battery.charge_limit_max_pct`, `battery.charge_rate_c`, optional `battery.charge_rate_soc_threshold_pct`), `telemetry` config (`poll_ms`, `retain_seconds`), and `TelemetrySample`
  - `service/src/state.rs`: shared `AppState` (locks, token, in‑memory `telemetry_samples`)
  - Background tasks (`service/src/tasks`): `power` (TDP + robust thermal limit with boot delay/reapply), `battery`, `fan_curve`, `auto_update`, `telemetry`
  - CLI wrappers (`service/src/cli`): `framework_tool.rs`, `ryzen_adj.rs`
  - Utilities (`service/src/utils`): `github`, `download`, `wget`, `fs`, etc.
  - `service/src/static.rs`: static file serving for the UI
  - `service/src/shortcuts.rs`: Windows shortcut creation logic (Edge/Chrome/Brave app mode + .url fallback)

### Frontend Web UI (Svelte)

- Settings: `web/src/components/SettingsModal.svelte` — service update controls, DaisyUI theme selector (all built‑in themes) with instant apply. Theme preference persists to backend config (`Config.ui.theme`) and is applied on startup. Desktop shortcut installer included.

- Entry: `web/src/App.svelte` (@App.svelte) — polls `/health`; `flex-wrap` layout.
- Panels: `Sensors` (temperature graphs from `/api/thermal/history`), `Power` (AC/Battery profiles; shows live TDP/thermal), `Battery` (charge/health and limits), `FanControl` (Auto/Manual/Curve with header selector).
  - Battery panel info bar now shows live charge/discharge current, computed C‑rate, pack voltage, SoC, battery health, and max charge limit in a compact header card that mirrors the Power panel layout.
- Device header: `web/src/components/DeviceHeader.svelte` shows basic system info (CPU, dGPU if present, RAM, OS, BIOS, and screen resolution) with a compact grid of labeled spec chips to align with the rest of the dashboard. Screen resolution is derived from the native panel resolution (physical pixels) by compensating for OS/browser scaling. On mobile, the device image is a small inline thumbnail (no separate column) to reduce header height. Connection status is a compact circular button with a hover tooltip to save space on narrow screens.
- Graph shell: `web/src/components/GraphPanel.svelte` standardizes spacing and sticky settings; used by `Sensors` and Fan Control (Curve).
- Tooltips: `web/src/lib/tooltip.ts` (portaled, auto‑flip). Includes built‑in outside‑click and Escape dismiss (default on; opt‑out with `attachGlobalDismiss: false`). Emits `dismiss` event to sync local state. DaisyUI tooltip usage removed.
- App placeholders: When the service is unhealthy or the CLI is missing, panel placeholders now include small contextual icons (thermometer, fan, power, battery) for quick visual scanning.
- MultiSelect: per‑instance IDs and auto left/right alignment.
- Device header: static images (no crossfade/width/pulse).
- API client: generated (`web/src/api/*`). Use `DefaultService` and `OpenAPI` for all requests.
 - `UiSlider.svelte`: standard slider card with label/value and optional enabled toggle; supports a header `trailing` slot for chips/menus (e.g., SoC threshold chip in Battery Rate control). The header shows the raw slider value (no rounding).

### Things to Pay Attention To
- Always use the generated API client (`DefaultService`, `OpenAPI`) for all requests.
- Do NOT call `fetch` directly to backend endpoints in UI code.
- Prefer typed responses from OpenAPI models NEVER EVER use fetch() when interacting with the backend service.
- For authenticated calls, pass `Bearer ${OpenAPI.TOKEN}`.
- To reflect backend changes, rebuild the service to refresh `openapi.json`, then run `npm run gen:api` in `web/`.
- Hosted vs Embedded: when hosted and an update is available, `VersionMismatchModal.svelte` blocks with actions to open the local app or download the installer (`VITE_INSTALLER_URL`). Embedded mode (127.0.0.1) continues normally.

- Env: `web/.env.local`
  - `VITE_API_BASE` (defaults to `http://127.0.0.1:8090`)
  - `VITE_CONTROL_TOKEN` (bearer token for write ops)
  - `VITE_INSTALLER_URL` (MSI URL for "Download Service" button)
- Build/dev:
  - `npm i && npm run dev` (dev)
  - `npm run build` (generates `web/dist` used by service/static)
  - Embedded UI: Cargo feature `embed-ui` (default) embeds `web/dist`; CI disables it for OpenAPI generation.

### Installation & Packaging

- MSI assets in `service/wix/*` (built via `web/scripts/build-msi.mjs`).
- Shortcuts: created on demand via `/api/shortcuts/create`; auto‑update preserves original shortcut choice.
- Updates: `GET /api/update/check`, `POST /api/update/apply` (Windows `msiexec`).
- MSI injects env values into the service (allowed origins, token, port, update repo).

### Configuration

- Persisted at `C:\ProgramData\FrameworkControl\config.json`
- Fan modes: Auto, Manual duty, Curve (`sensors: string[]`, service applies max across selected sensors)
- Telemetry: `telemetry.poll_ms`, `telemetry.retain_seconds` (history for `/api/thermal/history`)
- Writes require `FRAMEWORK_CONTROL_TOKEN` (Bearer)
- Updates: `FRAMEWORK_CONTROL_UPDATE_REPO` used by update endpoints; MSI build reads tokens from env/CLI

### Developer Quick Start

- Backend (dev):
  - `cd framework-control/service`
  - Set `.env` with:
    - `FRAMEWORK_CONTROL_ALLOWED_ORIGINS`
    - `FRAMEWORK_CONTROL_TOKEN`
    - `FRAMEWORK_CONTROL_PORT=8090`
  - `cargo run`
- Frontend (dev):
  - `cd framework-control/web`
  - `.env.local` with `VITE_API_BASE`, `VITE_CONTROL_TOKEN`
  - `npm i && npm run dev`

### Notable Cross-Repo Context

- `framework-system`: houses `framework_tool` and `framework_lib`
- `inputmodule-rs`: firmware and tooling for Framework 16 input modules (e.g., `qtpy/src/main.rs` @main.rs for USB CDC commands + LED control)
- `RyzenAdj`: third-party CLI to adjust AMD Ryzen power/thermal parameters; downloaded from GitHub releases when missing.

### Roadmap (per README)

- TDP control, telemetry dashboards, LED matrix support, additional EC controls, Linux support, import/export.

### Implementing: Battery Panel

- UI: `web/src/components/BatteryControl.svelte` (compact, same height as Power panel). Layout mirrors Power panel: top status bar card + two separate slider cards (Max Charge Limit, Rate (C)) with a small gap between them. SoC threshold input is compact with a top label and auto‑applies changes (throttled).
- Enhancement: Battery Rate slider uses the `trailing` slot to show a subtle SoC threshold chip; clicking opens a tiny anchored popover with a number field and presets (auto‑apply with throttle, Escape/outside click to close).
- Rate Limit specifics: Rate slider now has an Enabled toggle (mirrors Power sliders). When disabled, the UI approximates “no limit” by applying 1.0 C. The header shows the value as-is.
- Data: GET `/api/power` supplies AC presence, battery stats (percentage, voltage/current, cycle count, LFCC, design capacity), and charge limit min/max.
 - Config-driven:
  - Read from `GET /api/config` → `config.battery`
  - Write via `POST /api/config` with partial: `{ "battery": { "charge_limit_max_pct": { "enabled": bool, "value": number }, "charge_rate_c": { "enabled": bool, "value": number }, "charge_rate_soc_threshold_pct"?: number } }` (omitting `charge_rate_soc_threshold_pct` clears/unsets the SoC threshold)
  - Background task applies on change and every 30 minutes
- Notes:
  - EC only exposes GET for charge limit min/max; rate limit is set‑only (persist last-set values in service config for UI echo).
  - Battery health in the UI is computed client-side from last full charge capacity vs design capacity plus cycle count (no additional backend fields).
  - All write endpoints require `Authorization: Bearer <token>`.

### How to Update This Summary

- When adding endpoints: list under Backend → Routes with method/path and brief description
- When adding UI panels/components: list under Frontend → Components and mention which endpoint(s) it consumes
- When adding background tasks: list under Backend → Other key files
- Keep env variables and config surface in sync with code
