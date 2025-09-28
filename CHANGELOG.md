## Unreleased

- Power control:
  - Added `power` section to config with `tdp_watts`, `thermal_limit_c`.
  - New background task applies power settings at boot and on config changes.
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
