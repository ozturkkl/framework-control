## Unreleased

- Backend: Add minimal 2s TTL cache to reduce CLI load in some cases. Cache is returned only within TTL; cache is cleared and errors are propagated on failures.
- Web: New `Telemetry.svelte` mini-card in Telemetry panel showing live TDP (W), Thermal (°C), and Battery % with charging state;
- Power config/UI:
  - Power config redesigned with AC/Battery profiles and per-setting enabled flags.
  - Power task selects profile based on AC presence and applies enabled values.
- Backend: Power task now includes a conservative TDP reapply mechanism (quiet-window + cooldown, with tolerance) that reads current TDP from `ryzenadj --info` and re-applies the user preference only when the OS/driver adjustments have settled. This improves reliability without fighting calibration behavior.
- Install on demand, notify user if install is not completed due to anti-virus false positive.
- Windows shortcuts: Brave app-mode support with `.url` fallback; improved detection and status.
- Backend: refactor CLI wrapper into `service/src/cli/` module (`framework_tool.rs`), preparing for additional CLIs (e.g., RyzenAdj, inputmodule-rs).
- Backend: add `service/src/utils/` with `github.rs` and `wget.rs` for shared release/winget helpers.
- Breaking: `/api/power`, `/api/version` and `/api/thermal` now return parsed JSON (typed)
- Windows installer: validate `framework_tool` binary after WinGet resolve and fall back to direct download if the WinGet link is broken.
- Improved detection of missing `framework_tool`. Just in time state resolver to keep the state updated on changes.
- Other minor bug fixes & improvements.

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
