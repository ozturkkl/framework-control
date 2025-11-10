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
- Other key files:
  - `service/src/config.rs`: load/save config JSON in `C:\ProgramData\FrameworkControl\config.json`
  - `service/src/types.rs`: API/request/response and config types (power config now has AC/Battery profiles with enabled flags). Adds `telemetry` config (`poll_ms`, `retain_seconds`) and telemetry history response types.
  - `service/src/state.rs`: shared `AppState` (config RwLock, CLI handle, auth helper)
  - `service/src/static.rs`: static file serving for the UI
  - `service/src/shortcuts.rs`: Windows shortcut creation logic (Edge/Chrome/Brave app mode + .url fallback)
- `service/src/cli/ryzen_adj.rs`: RyzenAdj wrapper and on-demand install helper
  - `service/src/tasks/*`: background tasks (apply fan curve; apply power settings; auto-update checks; telemetry history collector)
  - `service/src/tasks/telemetry.rs`: polls `framework_tool --thermal` at `telemetry.poll_ms` and retains recent samples in-memory (trimmed by `telemetry.retain_seconds`).
  - `service/src/tasks/fan_curve.rs`: fan control task supports sensors-only curves (max of selected sensors).
  - `service/src/tasks/power.rs`: selects AC/Battery profile based on AC presence and applies enabled TDP/thermal values. Adds a "sticky but patient" TDP reapply: polls current TDP via `ryzenadj --info`, waits for a quiet window (no drift) before reapplying, and rate-limits reapply attempts. This avoids fighting the OS/driver's gradual adjustments while keeping the user's requested TDP in effect.
  - `service/src/cli/`: CLI integrations namespace
    - `framework_tool.rs`: wrapper for `framework_tool` (resolution, install, helpers)
    - `ryzen_adj.rs`: wrapper for `ryzenadj` (resolution, GitHub releases download, helpers)
    - `mod.rs`: re-exports `FrameworkTool` and `RyzenAdj`
  - `service/src/utils/`: shared helpers
    - `github.rs`: GitHub repo/release helpers (fetch, parse, asset selection)
    - `download.rs`: download utilities. `download_to_path(url, root_dir)` now takes a root directory and returns the final created path (dir for zips, file for non-zips). The low-level raw file helper is internal-only.
    - `wget.rs`: winget resolution and install helpers (Windows)
    - `fs.rs`: filesystem helpers (e.g., `copy_dir_replace(src, dst)`)
- CLI dependency: `framework_tool` (from `framework-system`). Service wraps it rather than linking low-level driver libraries directly.

### Frontend Web UI (Svelte)

- Entry: `web/src/App.svelte` (@App.svelte)
  - Polls `/health` every second to update `healthy` + `cliPresent`
- Renders panels: Sensors, Power, Fan Control; responsive layout using simple `flex-wrap` to preserve source order and allow dynamic heights. Mobile: single column; wide: two columns when healthy, three columns when not healthy. Animations removed so cards snap into place.
  - `FanControl.svelte` contains the Auto/Manual/Curve mode selector, overlaid in the panel header area (like PowerControl).
  - `DeviceHeader.svelte`: removed image crossfade, width transition, and pulsing update dot animation for a snappier, static header.
- API client: `web/src/api/*` generated from OpenAPI (`scripts/gen-api.mjs`)
- Components: `web/src/components/*` (`DeviceHeader.svelte`, `Panel.svelte`, `FanControl.svelte`)
  - `PowerControl.svelte`: power controls with AC/Battery tabs; per-setting Enabled checkbox; compact layout
    - Battery mode: TDP slider visually shows an unreachable segment beyond 60 W (error tint) while the actual range remains 5–120 W; input is clamped to 60 W on battery.
    - Remembers last selected AC/Battery tab via `localStorage` key `fc.power.activeProfile`.
    - Intel gating: Uses `/api/system` to detect CPU vendor and shows an AMD-only notice on Intel systems (RyzenAdj-based controls not supported on Intel yet).
    - Adds a "Remove helper" action to uninstall the downloaded RyzenAdj via `POST /api/ryzenadj/uninstall`.
- Tooltips: preferred minimal element-based action is `web/src/lib/tooltip.ts`. Attach it to the tooltip content element; pass an `anchor` (Element or fn returning Element) and a `visible` boolean to control show/hide. It portals to `document.body`, uses fixed positioning, auto-flips above/below, clamps within the viewport, and follows anchor changes (MutationObserver + rAF).
  - Legacy `web/src/lib/tooltipPortal.ts` has been removed after migrating all usages.
  - Example:
    ```svelte
    <script lang="ts">
      import { tooltip } from "$lib/tooltip";
      let btn: HTMLElement; let visible = false;
    </script>
    <button bind:this={btn} on:mouseenter={() => visible = true} on:mouseleave={() => visible = false}>
      Hover
    </button>
    <div use:tooltip={{ anchor: btn, visible }} class="rounded bg-base-100 border px-2 py-1 text-sm">
      Hello tooltip
    </div>
    ```
- Sensors panel (`Sensors.svelte`): multi-sensor temperature graph backed by `/api/thermal/history` with sensor selector and backend poll-interval control.
- Sensors graph includes a hover crosshair and nearest-point tooltip with sensor name, value, and relative time.
- `GraphPanel.svelte`: thin wrapper that standardizes card spacing, top/bottom areas, and the sticky settings pane with matched height. Exposes `top`, `graph`, `bottom`, `settings`, and `settings-top-right` slots, plus `openSettings`/`closeSettings` slot props. Used by `Sensors.svelte` and the Curve mode of `FanControl.svelte`.
- Spacing: Telemetry and Fan Control chart cards share the same padded header/body/footer layout for consistent side‑by‑side alignment (via `GraphPanel.svelte`).
- Settings panes in `Sensors.svelte` and `FanControl.svelte` maintain the same height as their graph panels; the content is vertically centered when shorter and becomes scrollable when taller to avoid panel layout shifts (handled by `GraphPanel.svelte`).
- Panels: Consistent header-to-content spacing. Header adds a bottom margin; the first child inside panel content has top margin/padding reset to 0 for uniformity (no per-panel top spacing hacks).
- `TelemetryGraph.svelte`: merged into `Sensors.svelte` (removed).
  - `SettingsModal.svelte`: adds Updates section to check/apply service updates
  - `MultiSelect.svelte`: daisyUI-based multi-select (dropdown + checkboxes) with an optional right-aligned `itemRight` slot per option (used to display live sensor temperatures in `FanControl.svelte`). Each instance now generates a unique ID prefix to avoid cross-instance input/label collisions.
    - Adds dynamic edge-aware alignment: dropdown flips between left/right to stay within the viewport when near edges.
    - Fix: Prevents invisible overlay after closing by gating visibility/pointer-events on open state and resetting alignment on close.
- Shared utilities: `web/src/lib/*`
- Fan Control curve editor shows a compact tooltip next to the focused/dragged point with a colored indicator and bold values (e.g., `30°C · 30%`), avoiding brackets and clamped within the graph bounds.
- Focused curve points support arrow-key nudging (Shift=±5, Ctrl=±10). Home/End jump to min/max temperature.
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
  - `VITE_INSTALLER_URL` (MSI URL for "Download Service" button)
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
  - Curve config: `sensors: string[]` (no defaults; empty means skip applying and log, the front end will try to populate after fetching). The UI displays the latest temperature next to each sensor in the selector.
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
- `RyzenAdj`: third-party CLI to adjust AMD Ryzen power/thermal parameters; downloaded from GitHub releases when missing.

### Roadmap (per README)

- TDP control, telemetry dashboards, LED matrix support, additional EC controls, Linux support, import/export.

### How to Update This Summary

- When adding endpoints: list under Backend → Routes with method/path and brief description
- When adding UI panels/components: list under Frontend → Components and mention which endpoint(s) it consumes
- When adding background tasks: list under Backend → Other key files
- Keep env variables and config surface in sync with code
