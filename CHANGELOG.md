## Unreleased

- Windows shortcuts: Brave app-mode support with `.url` fallback; improved detection and status.
- Backend: refactor CLI wrapper into `service/src/cli/` module (`framework_tool.rs`), preparing for additional CLIs (e.g., RyzenAdj, inputmodule-rs).
- Backend: add `service/src/utils/` with `github.rs` and `wget.rs` for shared release/winget helpers.

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


