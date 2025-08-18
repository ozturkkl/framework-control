# Framework Control

Framework Control is a lightweight control surface for Framework laptops. It exposes a minimal local HTTP API and a modern web UI to monitor telemetry and tweak core platform settings (fans, power, charging, etc.). The project is designed to be fast, unobtrusive, and extensible. Windows background service + Svelte web UI for basic telemetry and fan control on Framework laptops.

## DEMO

1. Open the web app: [https://ozturkkl.github.io/framework-control/](https://ozturkkl.github.io/framework-control/)
2. Install the background service that allows the web app to talk to the low level CLI (download link provided in the web app)

## Goals

- Minimal always‑on local service with a clean REST API
- Fan controls: manual duty/RPM now; editor for curves soon
- Telemetry surface: AC/battery/charge info; temps/fans; later PD/ports, expansion bay, input deck
- Persisted settings with sensible defaults and easy backup/restore
- Packaging suitable for end users (no terminals required)
- Design for Linux parity later

## Stretch goals

- LED Matrix support (Framework 16 input module)
  - Live canvas editor (draw/erase/brightness/sleep) inspired by the public tool at https://ledmatrix.frame.work/
  - One/two‑module layouts (9×34 and 9×68), dithering and content scheduling
  - Optional integrations for animations/GIF/pixel art
- Additional EC‑exposed toggles: keyboard backlight, fingerprint LED levels, input deck modes, PD/ports deep dive, EC console tail, etc.

## Architecture (MVP)

- Backend service: Rust (Tokio + Poem + poem-openapi)
  - Exposes a tiny HTTP API (default: http://127.0.0.1:8090)
  - Generates OpenAPI on demand
  - Executes Framework CLI (`framework_tool`) for all EC interactions (no direct library bindings)
  - Returns raw CLI stdout in a simple JSON envelope; the web UI parses as needed
  - Applies a persisted fan-control config at boot (auto/manual/curve)
- Frontend UI: Svelte + Vite
  - Runs locally (dev: http://127.0.0.1:5174) and talks to the backend API
  - Simple pages: Telemetry and Fan control
- Packaging: WiX
  - MSI installs the service binary to `C:\Program Files\FrameworkControl` and registers the background process with proper elevation at boot (no interactive consoles required)

## Why CLI‑only for EC?

Early iterations used the Rust `framework_lib` directly. On Windows that required build‑time git metadata and custom driver bindings, which added fragility to packaging and dev setup. Pivoting to the official CLI (`framework_tool`) gives a stable, tested interface with consistent elevation semantics on Windows. It also maps cleanly to Linux later.

## CLIs we use (and plan to)

- Framework EC CLI: `framework_tool`
  - Power/charge telemetry and battery info
  - Fan duty/RPM and auto mode
  - Other EC‑exposed toggles (keyboard backlight, etc.)
- Input module control (future): `inputmodule-control` for LED Matrix
  - Drawing, images, games, brightness, sleep/wake over USB CDC‑ACM

The app expects these CLIs to be present for the associated features. The Windows installer (WiX) validates and can guide installation if needed.

## API (MVP)

- `GET /api/health` → "ok"
- `GET /api/power` → `{ ok: boolean, stdout?: string, error?: string }` (raw output of `framework_tool --power`)
- `GET /api/thermal` → `{ ok: boolean, stdout?: string, error?: string }` (raw output of `framework_tool --thermal`)
- `GET /api/versions` → `{ ok: boolean, stdout?: string, error?: string }` (raw output of `framework_tool --versions`)
- `GET /api/system` → `{ ok: boolean, cpu: string, memory_total_mb: number, os: string, dgpu?: string }`
- `GET /api/config` → `{ ok: boolean, config: Config }`
- `POST /api/config` → `{ ok: boolean }` (body: `PartialConfig`; currently `{ fan_curve?: FanCurveConfig }`; merged and persisted)

Default bind: `127.0.0.1:8090`. The UI reads this via a simple `API_BASE` config.

### Config

- Location (Windows): `C:\ProgramData\FrameworkControl\config.json` (override with `FRAMEWORK_CONTROL_CONFIG`)
- Shape (subset shown):

```json
{
  "fan_curve": {
    "enabled": false,
    "mode": "auto", // "auto" | "manual" | "curve"
    "sensor": "APU",  // or "CPU"
    "points": [[40,0],[60,40],[75,80],[85,100]],
    "poll_ms": 2000,
    "hysteresis_c": 2,
    "rate_limit_pct_per_step": 100,
    "manual_duty_pct": null
  }
}
```

- Behavior:
  - When `enabled=false` or `mode="auto"`, the service ensures platform auto fan control (`--autofanctrl`).
  - When `mode="manual"` and `manual_duty_pct` is set, the service applies `--fansetduty`.
  - When `mode="curve"`, the service applies a piecewise-linear curve with hysteresis and optional rate limit. Rate limit constrains per-step duty change by `rate_limit_pct_per_step`.

### Server

- Bind address can be overridden with env vars:
  - `FRAMEWORK_CONTROL_HOST` (default `127.0.0.1`)
  - `FRAMEWORK_CONTROL_PORT` (default `8090`)
- Startup log is written to `C:\Program Files\FrameworkControl\service.log` (or a temp fallback) to aid troubleshooting.

## Developer setup

- Rust: stable toolchain
- Node.js for the web UI

Dev commands:

```bash
# Backend (dev)
cd framework-control/service
cargo run

# Frontend (dev)
cd framework-control/web
npm i
npm run dev
```

Packaging (Windows / WiX): build the MSI to install the service to `C:\Program Files\FrameworkControl` and register it for auto‑start with elevation.

### Dev environment variables

Service reads a local `.env` on startup (via dotenvy). Create `framework-control/service/.env`:

```
# Allow your dev UI origin(s)
FRAMEWORK_CONTROL_ALLOWED_ORIGINS=http://127.0.0.1:5174,http://localhost:5174

# Token required for write operations from the UI (Bearer token)
FRAMEWORK_CONTROL_TOKEN=<long-random-token>

# Optional: pick a different port for dev
# FRAMEWORK_CONTROL_PORT=8091
```

Web UI reads a `.env.local`. Create `framework-control/web/.env.local`:

```
# Local service URL (defaults to http://127.0.0.1:8090 if omitted)
VITE_API_BASE=http://127.0.0.1:8090

# Same token as the service uses so the client sends Authorization: Bearer <token>
VITE_CONTROL_TOKEN=<long-random-token>
```

Notes:
- The service always binds to loopback (`127.0.0.1`). It is not reachable from other machines.
- For CORS to pass in dev, the service must list your dev origin(s) in `FRAMEWORK_CONTROL_ALLOWED_ORIGINS`.
- Write routes require the bearer token. The UI must provide `VITE_CONTROL_TOKEN`.

## OpenAPI + client generation

- The service can emit an OpenAPI spec and exit:

```bash
# From repository root
cd framework-control/service
cargo run -- --generate-openapi  # writes ../web/openapi.json
```

- The web app includes a helper:

```bash
cd framework-control/web
npm run gen:api
```

Notes:
- Generation runs `cargo run -- --generate-openapi` with an isolated cargo target (`service/target/openapi`) so it does not conflict with a running `cargo run` in another terminal.
- The generated TypeScript client lives in `web/src/api` and is updated on `postinstall`, `predev`, and `prebuild`.

Auth in the generated client:
- The `POST /api/config` endpoint expects an `Authorization` header. The client will attach `Authorization: Bearer <token>` if `OpenAPI.TOKEN` is set (from `VITE_CONTROL_TOKEN`).

## Frontend API base

- By default, requests are sent to `OpenAPI.BASE` which is set in `web/src/main.ts`:
  - `VITE_API_BASE` if provided, otherwise `http://127.0.0.1:8090`.
- Alternatively, you can remove that line and configure a Vite dev proxy to the backend (e.g. proxy `/api` → `http://127.0.0.1:8090`).

## GitHub Pages deployment

The web UI is deployed to GitHub Pages under `/framework-control/`. Assets use `import.meta.env.BASE_URL` so they resolve correctly.

To allow the Pages UI to talk to your local service:
- The service must allow the Pages origin and require a token.
- At runtime on your PC, the Windows service process receives these env vars from the installer (MSI) via WinSW configuration.

CI secrets used to stamp MSI env:
- `ALLOWED_ORIGINS`: e.g. `https://ozturkkl.github.io`
- `CONTROL_TOKEN`: a long random token

Configure them in GitHub → Settings → Secrets and variables → Actions.

The workflow `.github/workflows/release-service.yml` replaces placeholders in `service/wix/FrameworkControlService.xml` so the installed service runs with:
- `FRAMEWORK_CONTROL_ALLOWED_ORIGINS=%ALLOWED_ORIGINS%`
- `FRAMEWORK_CONTROL_TOKEN=%CONTROL_TOKEN%`

Frontend token for Pages:
- During the Pages build, set `VITE_CONTROL_TOKEN` (for example via Pages build secrets) so the client includes the bearer token.
- Alternatively, prompt the user to paste their local token once and store it in `localStorage`, then set `OpenAPI.TOKEN` on load.

## Roadmap

- Fan curve editor with hysteresis and rate limiting
- WebSocket for live telemetry streaming
- Settings persistence + import/export
- LED Matrix editor and content pipeline
- Linux support (systemd unit; udev rules for input modules)

## Notes

- Security: API binds to loopback only; no remote exposure in MVP
- Logs: the service writes a small startup log to `C:\Program Files\FrameworkControl\service.log` (or a temp fallback) to help diagnose PATH/elevation issues when running the CLI
