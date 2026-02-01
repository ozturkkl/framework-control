# Linux Installation Guide

Framework Control on Linux runs as a systemd service and requires the official Framework CLI (`framework_tool`) to communicate with the embedded controller.

## Quick Install (Recommended)

Run the automated installer as root:

```bash
curl -fsSL https://raw.githubusercontent.com/ozturkkl/framework-control/main/install-linux.sh | sudo bash
```

This will:
- Download the latest release from GitHub
- Install the binary to `/usr/local/bin/framework-control`
- Install and enable the systemd service
- Start the service automatically
- Verify the installation

After installation, access the web UI at the address shown in the installer output.

## Prerequisites

### 1. Install `framework_tool`

The service requires the official Framework CLI to be installed and available on your `PATH`.

Follow Framework's official installation guide:
https://github.com/FrameworkComputer/framework-system#installation

Verify it works:

```bash
framework_tool --versions
```

### 2. System Requirements

- **systemd**: Required for service management
- **curl**: For downloading releases
- **tar**: For extracting the release tarball
- **Root access**: The service must run with elevated permissions

## Manual Installation

If you prefer to install manually:

### 1. Download the latest release

```bash
cd /tmp
curl -fsSL -o framework-control.tar.gz \
  https://github.com/ozturkkl/framework-control/releases/latest/download/framework-control-service-x86_64.tar.gz
```

### 2. Extract and install

```bash
tar -xzf framework-control.tar.gz
sudo install -m 755 framework-control /usr/local/bin/
sudo install -m 644 framework-control.service /etc/systemd/system/
```

### 3. Enable and start the service

```bash
sudo systemctl daemon-reload
sudo systemctl enable framework-control.service
sudo systemctl start framework-control.service
```

### 4. Verify it's running

```bash
sudo systemctl status framework-control.service
# Check logs for the actual port, then:
curl http://127.0.0.1:<port>/api/health
```

## Configuration

The service stores its configuration at `/etc/framework-control/config.json`.

This file is created automatically by the service and contains:
- Fan control settings (mode, curve, calibration)
- Power profiles (AC/Battery TDP and thermal limits)
- Battery charge limits and rates
- Telemetry settings
- UI theme preferences

You don't need to edit this file manually - all settings are managed through the web UI.

## Service Management

### Check status
```bash
sudo systemctl status framework-control.service
```

### View logs
```bash
sudo journalctl -u framework-control.service -f
```

### Restart service
```bash
sudo systemctl restart framework-control.service
```

### Stop service
```bash
sudo systemctl stop framework-control.service
```

### Disable auto-start
```bash
sudo systemctl disable framework-control.service
```

## Desktop Integration

After the service is running, you can create a desktop shortcut from within the web UI:

1. Open the web UI (check installer output or service logs for the URL)
2. Click the settings icon (⚙️)
3. Under "Desktop Integration", click "Create Shortcuts"

This will add a desktop entry to `~/.local/share/applications/framework-control.desktop` that opens the UI in your default browser.

## Updates

Framework Control includes an auto-update system:

1. The service checks for updates automatically (configurable in Settings)
2. When an update is available, you'll see a notification in the UI
3. Click "Install Update" to download and install the new version
4. The service will restart automatically

You can also update manually by re-running the install script.

## Uninstallation

To remove Framework Control:

```bash
sudo systemctl stop framework-control.service
sudo systemctl disable framework-control.service
sudo rm /etc/systemd/system/framework-control.service
sudo rm /usr/local/bin/framework-control
sudo rm -rf /etc/framework-control
sudo systemctl daemon-reload
```

Optionally remove the desktop entry:
```bash
rm ~/.local/share/applications/framework-control.desktop
```

## Troubleshooting

### Service won't start

Check the logs:
```bash
sudo journalctl -u framework-control.service -n 50
```

Common issues:
- **framework_tool not found**: Ensure it's installed and on `PATH`
- **Permission denied**: The service must run as root
- **Port already in use**: Another service may be using the configured port (check `FRAMEWORK_CONTROL_PORT`)

### UI says "framework_tool is missing"

The service can start without `framework_tool`, but it won't be able to interact with the EC.

Verify `framework_tool` is accessible to the service user:
```bash
sudo which framework_tool
sudo framework_tool --versions
```

If it's not found, ensure it's installed system-wide and not just for your user.

### Health check fails

The service may still be starting up. Wait a few seconds and try again:
```bash
# First, find the port from logs:
sudo journalctl -u framework-control.service | grep "listening on"
# Then check health:
curl http://127.0.0.1:<port>/api/health
```

If it continues to fail, check the service logs.

## Distribution-Specific Notes

### NixOS

The installer should work on NixOS as `/usr/local/bin` is writable. However, for a more NixOS-native approach, consider packaging it as a Nix derivation.

### Immutable Distributions (Fedora Silverblue, Bazzite, etc.)

The installer uses `/usr/local/bin` which is typically writable on immutable systems. If you encounter issues, you may need to use `rpm-ostree` or your distribution's preferred method for adding system binaries.

### Arch Linux

All prerequisites (`curl`, `tar`, `systemd`) are available in the base system or easily installable via `pacman`.

## Security Notes

- The service binds to `127.0.0.1` only - it's not accessible from the network
- Write operations require bearer token authentication (baked into the binary during build)
- The service runs as root because `framework_tool` requires elevated permissions for EC access

## Support

For issues, feature requests, or questions:
- GitHub Issues: https://github.com/ozturkkl/framework-control/issues
- Project README: https://github.com/ozturkkl/framework-control