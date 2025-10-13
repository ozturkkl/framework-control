# Framework Control

Framework Control is a lightweight control surface for Framework laptops. It exposes a minimal local HTTP API and a modern web UI to monitor telemetry and tweak core platform settings (fans, power, charging, etc.). The project is designed to be fast, unobtrusive, and extensible. Windows background service + Svelte web UI for telemetry and advanced fan control (auto/manual/curve, hysteresis, rate limit, live RPM overlay with calibration).

## How To Use

> ⚠️ **Warning (Framework 16)**
> At minimum BIOS 3.07 is required for proper EC/fan control behavior.
> Install the beta BIOS before using this tool or it won't work.
> Announcement and downloads: https://community.frame.work/t/framework-laptop-16-ryzen-7040-bios-3-06-release-beta/73326

<img width="1353" height="940" alt="image" src="https://github.com/user-attachments/assets/98565ce4-1093-4b77-a726-75d47752e090" />

1. Open the web app: [https://ozturkkl.github.io/framework-control/](https://ozturkkl.github.io/framework-control/)
2. Install the background service that allows the web app to talk to the low level CLI (download link provided in the web app)
3. Launch via Start Menu/Desktop shortcuts or visit http://127.0.0.1:8090 in your browser

## Disclaimer

This software is provided "as is," without warranty of any kind, express or implied. By using it, you acknowledge that you are responsible for any configuration changes you make to your system and accept all risks associated with its use. The authors and contributors are not liable for any damage, data loss, hardware wear, unintended behavior, or other harm that may result from use or misuse of this application.

## Current Features

- **Fan Controls**: Auto, Manual duty, and Curve editor with hysteresis and rate‑limit
  - Live RPM overlay with crosshair showing current temperature and estimated duty% on the curve editor
  - One‑time calibration wizard for accurate RPM-to-duty mapping
- **Persistent Settings**: Configurations saved locally with sensible defaults
- **Clean Architecture**: Minimal always‑on local service with REST API (default: http://127.0.0.1:8090)
- **User-Friendly**: No terminal required - MSI installer for Windows with automatic service registration
- **Native App Experience**: Start Menu/Desktop shortcuts open Chrome/Edge in app mode (created on first run)

- **Power Controls (RyzenAdj)**:
  - Set TDP (applies STAPM/FAST/SLOW equally)
  - Set Tctl thermal limit

## Upcoming Goals

- **TDP Control**: Adjust processor power limits for performance/battery optimization
- **Telemetry Dashboard**: Real-time monitoring of AC/battery/charge info, temperatures, and fan speeds
- **LED Matrix Support** (Framework 16 input module):
  - Live canvas editor (draw/erase/brightness/sleep) inspired by https://ledmatrix.frame.work/
  - One/two‑module layouts (9×34 and 9×68), dithering and content scheduling
  - Optional integrations for animations/GIF/pixel art
- **Additional EC Controls**: Keyboard backlight, fingerprint LED levels, input deck modes
- **Linux Support**: systemd unit with udev rules for input modules
- **Import/Export**: Settings backup and sharing

## Architecture

- **Backend Service**: Rust (Tokio + Poem + poem-openapi)
  - Exposes HTTP API with OpenAPI generation
  - Executes Framework CLI (`framework_tool`) for all EC interactions
  - Executes RyzenAdj (`ryzenadj`) for AMD power/thermal adjustments
  - Applies persisted fan-control config at boot
- **Frontend UI**: Svelte + Vite
  - Responsive panel layout adapting to active fan mode
  - Real-time telemetry updates
- **Packaging**: WiX MSI installer for Windows (Linux support planned)

## Why CLI‑only for EC?

Early iterations used the Rust `framework_lib` directly. On Windows that required build‑time git metadata and custom driver bindings, which added fragility to packaging and dev setup. Pivoting to the official CLI (`framework_tool`) gives a stable, tested interface with consistent elevation semantics on Windows. It also maps cleanly to Linux later.

## Developer Setup

**Prerequisites**:

- Rust stable toolchain
- Node.js for the web UI

**Dev Commands**:

```bash
# Backend (dev)
cd framework-control/service
cargo run

# Frontend (dev)
cd framework-control/web
npm i
npm run dev
```

### Frontend API Usage Policy (Important)

- Always use the generated client in `web/src/api` (`DefaultService`, `OpenAPI`) for all calls to the local service.
- Do NOT use `fetch` directly for service endpoints anywhere in the UI.
- After changing backend routes or models, regenerate the client:
  1. Build the service to refresh `web/openapi.json`
  2. From `framework-control/web`, run: `npm run gen:api`

**Environment Variables**:

Service `.env` file (`framework-control/service/.env`):

```
# Allow your dev UI origin(s)
FRAMEWORK_CONTROL_ALLOWED_ORIGINS=http://127.0.0.1:5174,http://localhost:5174

# Token required for write operations from the UI
FRAMEWORK_CONTROL_TOKEN=<long-random-token>

# Required: port for the service
FRAMEWORK_CONTROL_PORT=8090
```

Web UI `.env.local` file (`framework-control/web/.env.local`):

```
# Local service URL (defaults to http://127.0.0.1:8090 if omitted)
VITE_API_BASE=http://127.0.0.1:8090

# Same token as the service
VITE_CONTROL_TOKEN=<long-random-token>
```

## Updates & Caching

- UI is served by the service; PWA is configured for passive offline caching (no auto‑reloads)
- When implementing update flow, trigger a one‑time reload after service update to pick up new UI

## API & Configuration

The service provides a REST API for telemetry (`/api/power`, `/api/thermal`, `/api/versions`), system info (`/api/system`), and config management (`/api/config`).

Configuration is stored in `C:\ProgramData\FrameworkControl\config.json` (Windows) and includes fan mode settings, curve points, calibration data, hysteresis, and rate limiting parameters.

## UI Features

### Fan Curve Editor

- Visual curve editing with drag-and-drop points
- Live RPM overlay with real-time crosshair showing current temperature and fan duty
- Guided calibration wizard for accurate RPM-to-duty% mapping
- Hysteresis and rate limiting controls to prevent oscillation
- Sensor selection (APU/CPU)

### Fan Modes

- **Auto**: Platform-managed fan control
- **Manual**: Fixed duty percentage slider
- **Curve**: Custom temperature-to-duty curve with advanced controls

## Security Notes

- API binds to loopback only (127.0.0.1) - no remote exposure
- Write operations require bearer token authentication
- Service logs startup info to `C:\Program Files\FrameworkControl\service.log` for diagnostics
