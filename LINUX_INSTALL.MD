# Linux Install

Framework Control on Linux expects the official Framework CLI (`framework_tool`) to be installed and available on your `PATH`.

## 1) Install `framework_tool`

Follow Framework’s official install steps:
https://github.com/FrameworkComputer/framework-system?tab=readme-ov-file#installation

Verify it works:

```bash
framework_tool --versions
```

## 2) Install + run Framework Control

- Install the Framework Control **service** (your distro/package/portable release).
- Start the service.
- Open the UI (hosted or local) and connect to `http://127.0.0.1:8090`.

## Troubleshooting

- If the UI says `framework_tool` is missing:
  - Confirm it’s on `PATH` for the service user:
    ```bash
    which framework_tool
    ```
  - If you can run it in your terminal but the service can’t find it, ensure the service is started with an environment that includes your `PATH` (common with systemd services).
