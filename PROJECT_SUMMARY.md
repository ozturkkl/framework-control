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
    - `GET /versions`: parsed versions (mainboard_type, uefi_version, etc.)
    - `GET /config`: return persisted config
    - `POST /config`: update config (requires `Authorization: Bearer <token>`)
    - `GET /system`: basic system info (CPU, memory, OS, dGPU guess)
    - `GET /shortcuts/status`: Start menu/Desktop shortcut existence
    - `POST /shortcuts/create`: create app-mode browser shortcuts (auth required)
    - `GET /update/check`: check for latest version from update feed (see env below)
    - `POST /update/apply`: install the update (auth required)
  - Helpers: GPU detection via PowerShell on Windows
- Other key files:
  - `service/src/config.rs`: load/save config JSON in `C:\ProgramData\FrameworkControl\config.json`
  - `service/src/types.rs`: API/request/response and config types
  - `service/src/state.rs`: shared `AppState` (config RwLock, CLI handle, auth helper)
  - `service/src/static.rs`: static file serving for the UI
  - `service/src/shortcuts.rs`: Windows shortcut creation logic (Edge/Chrome/Brave app mode + .url fallback)
  - `service/src/tasks/*`: background tasks (e.g., apply fan settings at boot)
  - `service/src/cli/`: CLI integrations namespace
    - `framework_tool.rs`: wrapper for `framework_tool` (resolution, install, helpers)
    - `mod.rs`: re-exports `FrameworkTool` and `resolve_or_install`
  - `service/src/utils/`: shared helpers
    - `github.rs`: GitHub repo/release helpers (fetch, parse, asset selection)
    - `wget.rs`: winget resolution and install helpers (Windows)
- CLI dependency: `framework_tool` (from `framework-system`). Service wraps it rather than linking low-level driver libraries directly.

### Frontend Web UI (Svelte)

- Entry: `web/src/App.svelte` (@App.svelte)
  - Polls `/health` every second to update `healthy` + `cliPresent`
  - Renders panels: Telemetry, Power, Fan Control; layout adapts to fan mode
  - Integrates `FanControl.svelte` for Auto/Manual/Curve config
- API client: `web/src/api/*` generated from OpenAPI (`scripts/gen-api.mjs`)
- Components: `web/src/components/*` (`DeviceHeader.svelte`, `Panel.svelte`, `FanControl.svelte`)
  - `SettingsModal.svelte`: adds Updates section to check/apply service updates
- Shared utilities: `web/src/lib/*`
- Frontend API usage guideline (do not bypass):

  - Always use the generated API client (`DefaultService`, `OpenAPI`) for all requests.
  - Do NOT call `fetch` directly to backend endpoints in UI code.
  - Prefer typed responses from OpenAPI models NEVER EVER use fetch() when interacting with the backend service.
  - For authenticated calls, pass `Bearer ${OpenAPI.TOKEN}`.
  - To reflect backend changes, rebuild the service to refresh `openapi.json`, then run `npm run gen:api` in `web/`.

- Hosted vs Embedded UI behavior:

  - The UI now detects when it is running as a hosted build (origin != local service origin) and the service has an update available.
  - In that case, a blocking modal (`VersionMismatchModal.svelte`) prevents using the hosted app to avoid version drift. It offers two actions: open the local app (http://127.0.0.1:PORT) and download the installer (if `VITE_INSTALLER_URL` is provided), or jump to Releases.
  - When served from the embedded service (127.0.0.1:PORT), normal operation continues. Updates are still surfaced in `Settings` and via the small indicator dot.

- Env: `web/.env.local`
  - `VITE_API_BASE` (defaults to `http://127.0.0.1:8090`)
  - `VITE_CONTROL_TOKEN` (bearer token for write ops)
- Build/dev:
  - `npm i && npm run dev` (dev)
  - `npm run build` (generates `web/dist` used by service/static)
  - Embedded UI feature flag: the Rust service now has an `embed-ui` Cargo feature (enabled by default) that embeds `web/dist` into the binary. CI OpenAPI generation runs with `--no-default-features` to avoid requiring `web/dist` at that step.

### Installation & Packaging

- MSI assets at `service/wix/*`, built via `web/scripts/build-msi.mjs` and service packaging
- Start Menu/Desktop shortcuts created on demand through `/api/shortcuts/create`
  - The MSI build injects env vars into the installed Windows service via token replacement in `service/wix/FrameworkControlService.xml`. Ensure these tokens are set when building:
    - `@ALLOWED_ORIGINS@` → `FRAMEWORK_CONTROL_ALLOWED_ORIGINS`
    - `@CONTROL_TOKEN@` → `FRAMEWORK_CONTROL_TOKEN`
    - `@CONTROL_PORT@` → `FRAMEWORK_CONTROL_PORT`
    - `@UPDATE_REPO@` → `FRAMEWORK_CONTROL_UPDATE_REPO`

### Configuration

- Persisted at `C:\ProgramData\FrameworkControl\config.json`
- Fan modes: Auto, Manual duty, Curve (with hysteresis, rate limiting, calibration)
- Write operations require `FRAMEWORK_CONTROL_TOKEN` (Bearer auth header)
- Updates:
  - `FRAMEWORK_CONTROL_UPDATE_REPO`: GitHub repo to check (format `owner/repo` or `https://github.com/owner/repo`). `GET /api/update/check` uses GitHub Releases API to fetch the latest tag and MSI asset URL; `POST /api/update/apply` downloads that MSI and launches `msiexec` on Windows.
- MSI build: pass tokens via CLI args or environment. `node web/scripts/build-msi.mjs --port 8090 --allowed-origins "http://127.0.0.1:5174" --token YOUR_TOKEN --update-repo owner/repo`. The script also reads `service/.env` for defaults.

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

### Roadmap (per README)

- TDP control, telemetry dashboards, LED matrix support, additional EC controls, Linux support, import/export.

### How to Update This Summary

- When adding endpoints: list under Backend → Routes with method/path and brief description
- When adding UI panels/components: list under Frontend → Components and mention which endpoint(s) it consumes
- When adding background tasks: list under Backend → Other key files
- Keep env variables and config surface in sync with code
