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

- Backend service: Rust (Axum + Tokio)
  - Exposes a tiny HTTP API (default: http://127.0.0.1:8090)
  - Executes Framework CLI (`framework_tool`) for all EC interactions (no direct library bindings)
  - Returns raw CLI stdout in a simple JSON envelope; the web UI parses as needed
  - Applies a persisted fan-control config at boot (auto/manual/curve)
- Frontend UI: Svelte + Vite
  - Runs locally (dev: http://127.0.0.1:5173) and talks to the backend API
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
- `GET /api/system` → `{ ok: boolean, cpu: string, memory_total_mb: number, os: string, gpus: string[], dgpu?: string }`
- `GET /api/config` → `{ ok: boolean, config: Config }`
- `POST /api/config` → `{ ok: boolean }` (body: `{ config: Partial<Config> }`; deep-merged and persisted)

Default bind: `127.0.0.1:8090`. The UI reads this via a simple `API_BASE` config.

### Config

- Location (Windows): `C:\ProgramData\FrameworkControl\config.json` (override with `FRAMEWORK_CONTROL_CONFIG`)
- Shape (subset shown):

```json
{
  "fan_curve": {
    "enabled": true,
    "mode": "curve", // "auto" | "manual" | "curve"
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
  - When `mode="curve"`, the service applies a piecewise-linear curve with hysteresis and optional rate limit.

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

## Roadmap

- Fan curve editor with hysteresis and rate limiting
- WebSocket for live telemetry streaming
- Settings persistence + import/export
- LED Matrix editor and content pipeline
- Linux support (systemd unit; udev rules for input modules)

## Notes

- Security: API binds to loopback only; no remote exposure in MVP
- Logs: the service writes a small startup log to `C:\Program Files\FrameworkControl\service.log` (or a temp fallback) to help diagnose PATH/elevation issues when running the CLI
