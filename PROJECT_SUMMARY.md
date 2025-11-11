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
    - `GET /power`: parsed power report (ac_present + optional battery info)
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
  - `service/src/types.rs`: API and config types; includes power AC/Battery profiles, `telemetry` config (`poll_ms`, `retain_seconds`), and `TelemetrySample`
  - `service/src/state.rs`: shared `AppState` (locks, token, in‑memory `telemetry_samples`)
  - Background tasks (`service/src/tasks`): `power`, `fan_curve`, `auto_update`, `telemetry`
  - CLI wrappers (`service/src/cli`): `framework_tool.rs`, `ryzen_adj.rs`
  - Utilities (`service/src/utils`): `github`, `download`, `wget`, `fs`, etc.
  - `service/src/static.rs`: static file serving for the UI
  - `service/src/shortcuts.rs`: Windows shortcut creation logic (Edge/Chrome/Brave app mode + .url fallback)

### Frontend Web UI (Svelte)

- Entry: `web/src/App.svelte` (@App.svelte) — polls `/health`; `flex-wrap` layout.
- Panels: `Sensors` (temperature graphs from `/api/thermal/history`), `Power` (AC/Battery profiles; shows live TDP/thermal), `FanControl` (Auto/Manual/Curve with header selector).
- Graph shell: `web/src/components/GraphPanel.svelte` standardizes spacing and sticky settings; used by `Sensors` and Fan Control (Curve).
- Tooltips: `web/src/lib/tooltip.ts` (portaled, auto‑flip). DaisyUI tooltip usage removed.
- MultiSelect: per‑instance IDs and auto left/right alignment.
- Device header: static images (no crossfade/width/pulse).
- API client: generated (`web/src/api/*`). Use `DefaultService` and `OpenAPI` for all requests.

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

### How to Update This Summary

- When adding endpoints: list under Backend → Routes with method/path and brief description
- When adding UI panels/components: list under Frontend → Components and mention which endpoint(s) it consumes
- When adding background tasks: list under Backend → Other key files
- Keep env variables and config surface in sync with code
