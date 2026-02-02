# Unreleased


## 0.5.0-beta.2 - 2026-01-26

- Service: Web server now starts immediately without waiting for `framework_tool` validation; CLI resolution/installation happens asynchronously in background, eliminating 60-second startup delay when tool is unavailable.
- Documentation: Removed hardcoded port references (8090) throughout documentation and scripts; install script now auto-detects the configured port from service logs and displays the actual URL.
- UI: Linux users now see a copy-able install command with one-click copy button instead of download link; manual install instructions and GitHub link provided as secondary actions.
- Linux: Added automated install script (`install-linux.sh`) that downloads latest release, installs binary to `/usr/local/bin`, sets up systemd service, and verifies installation with health check.
- Linux: Auto-update support with tar.gz extraction and in-place binary replacement (requires sudo/root permissions); service attempts automatic restart after update.
- UI: Update error messages now show specific backend error details (e.g., "installer asset not found") instead of generic text.
- Linux: Store config at `/etc/framework-control/config.json` instead of the Windows `ProgramData` path.
- Linux: Desktop entry support creates .desktop file in applications menu (~/.local/share/applications) using xdg-open. Detects actual user when service runs as root via SUDO_USER or active sessions.
- Linux: CI releases now build statically-linked binaries using musl target for compatibility with all Linux distributions; local builds remain dynamic for faster iteration.
- Build: Release artifacts now use stable naming (`framework-control-service-x86_64.msi` and `framework-control-service-x86_64.tar.gz`) for easier update system parsing.
- Build: Windows workflow now uses single runner calling `build-msi.mjs` for all steps (build web, build service, create MSI), reducing build time from ~8min to ~3min by avoiding redundant compilation across separate steps.
- Refactor: Renamed `zip_extract.rs` to `extract.rs` and added tar.gz extraction support; `download_to_path` now automatically extracts both zip and tar.gz archives using system tools.
- Removed macOS-specific code since Framework laptops don't support macOS (update system, UI icons, CLI helpers, comments).

## 0.4.3 - 2025-11-27

- Battery: Disabling max charge limit now performs a no-op instead of forcing 100%, allowing users to rely on their BIOS/EC charge limit configuration. Charge-rate (C) control now supports 0.05 as a minimum value to avoid users getting stuck at 0.
- Power: Fixed the max TDP unlock state not persisting after refresh; the UI now restores the correct unlock state when reloaded.

## 0.4.2 - 2025-11-24

- Battery: New Battery panel with live stats (health, C-rate, ETA) and configurable max charge limit / charge-rate (C) with SoC threshold; background task applies settings at boot.
- Power: `/api/power` now returns richer battery telemetry plus charge-limit info and is used by the power task to pick AC/Battery profiles based on AC presence.
- UI/Theme: In-app theme selector using DaisyUI themes (persisted via config and local storage) with early apply, plus updated device header imaging and more predictable tooltips.
- Sliders: Power/Battery sliders now support safe vs extended ranges, allow optional cap overrun, and display higher-precision values where useful.
- Tooltips: Unified global dismiss logic (outside click / Escape) and fixed a few cases where tooltips were either too sticky or closed unexpectedly.
- Fixed temp limit not applying on initial boot in some cases.
- Misc: Small power/battery header polish (better charger wattage display, ETA wording) and minor fan-curve interpolation/test tweaks. UI improvements, fixes.

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
