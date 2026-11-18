# Linux Installation Guide

Framework Control on Linux runs as a systemd service (or a runit service on Void Linux) and requires the official Framework CLI (`framework_tool`) to communicate with the embedded controller.

## Install Options

### Install script (systemd distributions and Void Linux)

Run the automated installer as root:

```bash
curl -fsSL https://raw.githubusercontent.com/ozturkkl/framework-control/main/install-linux.sh | sudo bash
```

To install the latest beta (falls back to stable if no beta exists):

```bash
curl -fsSL https://raw.githubusercontent.com/ozturkkl/framework-control/main/install-linux.sh | sudo bash -s -- --beta
```

This will:
- Download the latest release from GitHub (stable by default, or beta with `--beta`)
- Install the binary to `/usr/local/bin/framework-control`
- Install and enable the systemd service (or the runit service on Void Linux)
- Start the service automatically
- Verify the installation

After installation, access the web UI at the address shown in the installer output.

### Arch Linux (AUR)

An [AUR package](https://aur.archlinux.org/packages/framework-control) is available. It builds from source and installs the systemd unit and desktop entry. `framework_tool` is optional at install time - the service downloads it automatically on first run if it is not already on `PATH` (you can also install the official `framework-system` package beforehand).

With an AUR helper:

On a fresh install (or if dependency resolution fails), sync/update official repos first:

```bash
sudo pacman -Syu
```

Then install:

```bash
yay -S framework-control
# or
paru -S framework-control
```

A [beta/pre-release package](https://aur.archlinux.org/packages/framework-control-beta) is also published automatically when a GitHub prerelease is built (typically on version PRs). It conflicts with the stable package - install one or the other:

```bash
yay -S framework-control-beta
# or
paru -S framework-control-beta
```

### Debian / Ubuntu (apt)

> [!WARNING]
> If you installed Framework Control with the install script before, remove its service and binary first. Its unit in `/etc/systemd/system` would otherwise take precedence over the package's. These commands keep your settings in `/etc/framework-control`, which the uninstall script would delete:
>
> ```bash
> sudo systemctl disable --now framework-control
> sudo rm /etc/systemd/system/framework-control.service /usr/local/bin/framework-control
> sudo systemctl daemon-reload
> ```

Download and install the package from the latest release:

```bash
wget https://github.com/ozturkkl/framework-control/releases/latest/download/framework-control-service-x86_64.deb
sudo apt install ./framework-control-service-x86_64.deb
```

The package installs `/usr/bin/framework-control`, the systemd unit and a desktop entry, and enables and starts the service. It also adds the signed Framework Control apt repository (`/etc/apt/sources.list.d/framework-control.sources` with its key in `/usr/share/keyrings/`), so updates arrive through `apt upgrade`; in-app updates are turned off. Unattended upgrades only cover this repository if you add its origin (`framework-control`) to your unattended-upgrades configuration.

For betas, also add the beta repository next to the stable one (the beta channel also receives stable releases):

```bash
sudo wget -O /etc/apt/sources.list.d/framework-control-beta.sources https://ozturkkl.github.io/framework-control/packages/deb/beta/framework-control-beta.sources
sudo apt update && sudo apt upgrade
```

### Fedora / RHEL (dnf)

> [!WARNING]
> If you installed Framework Control with the install script before, remove its service and binary first. Its unit in `/etc/systemd/system` would otherwise take precedence over the package's. These commands keep your settings in `/etc/framework-control`, which the uninstall script would delete:
>
> ```bash
> sudo systemctl disable --now framework-control
> sudo rm /etc/systemd/system/framework-control.service /usr/local/bin/framework-control
> sudo systemctl daemon-reload
> ```

Install the package from the latest release:

```bash
sudo dnf install https://github.com/ozturkkl/framework-control/releases/latest/download/framework-control-service-x86_64.rpm
```

This works like the apt package above: it adds the signed repository as `/etc/yum.repos.d/framework-control.repo`, so updates arrive through `dnf upgrade`. The first update asks you to confirm importing the repository key; if you only update through GNOME Software, run `sudo dnf upgrade` once in a terminal to accept it.

For betas, also add the beta repository:

```bash
sudo curl -fsSL https://ozturkkl.github.io/framework-control/packages/rpm/beta/framework-control-beta.repo -o /etc/yum.repos.d/framework-control-beta.repo
sudo dnf upgrade
```

On Fedora Atomic (Silverblue, Kinoite, Bazzite), a package layered from a file never updates, so add the repository first and install by name:

```bash
sudo curl -fsSL https://ozturkkl.github.io/framework-control/packages/rpm/stable/framework-control-stable.repo -o /etc/yum.repos.d/framework-control.repo
sudo rpm-ostree install framework-control
```

Then reboot and enable the service with `sudo systemctl enable --now framework-control`.

### NixOS

`framework-control` is packaged in [nixpkgs](https://github.com/NixOS/nixpkgs/tree/master/pkgs/by-name/fr/framework-control) with a NixOS module. Add this to your configuration:

```nix
services.framework-control.enable = true;
```

Then rebuild:

```bash
sudo nixos-rebuild switch
```
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

- **systemd** or **runit** (Void Linux): Required for service management
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

### Void Linux (runit)

```bash
sudo sv status framework-control                    # check status
sudo tail -f /var/log/framework-control/current     # view logs
sudo sv restart framework-control                   # restart
sudo sv down framework-control                      # stop
sudo rm /var/service/framework-control              # disable auto-start
```

## Desktop Integration

After the service is running, you can create a desktop shortcut from within the web UI:

1. Open the web UI (check installer output or service logs for the URL)
2. Click the settings icon (⚙️)
3. Under "Desktop Integration", click "Create Shortcuts"

This will add a desktop entry to `~/.local/share/applications/framework-control.desktop` that opens the UI in your default browser.

## Updates

apt and dnf installs update through `apt upgrade` or `dnf upgrade`, so the in-app updater is turned off for them. Install script installs include an auto-update system:

1. The service checks for updates automatically (configurable in Settings)
2. When an update is available, you'll see a notification in the UI
3. Click "Install Update" to download and install the new version
4. The service will restart automatically

You can also update manually by re-running the install script.

## Uninstallation

### Install script installs

Run the automated uninstaller as root:

```bash
curl -fsSL https://raw.githubusercontent.com/ozturkkl/framework-control/main/uninstall-linux.sh | sudo bash
```

This stops and disables the service, then removes the binary, systemd unit (or runit service and logs),
config directory, desktop entry, and icon assets.

### AUR installs

```bash
sudo pacman -R framework-control
# or, if using the beta package:
sudo pacman -R framework-control-beta
```

The package hooks stop and disable the systemd service before removal.

### apt and dnf installs

```bash
sudo apt remove framework-control
# or
sudo dnf remove framework-control
```

This stops and disables the service and removes the downloaded `framework_tool` and the stable repository. If you added the beta repository, delete its file from `/etc/apt/sources.list.d/` or `/etc/yum.repos.d/` yourself.

On Fedora Atomic, run `sudo rpm-ostree uninstall framework-control`, delete `/etc/yum.repos.d/framework-control.repo`, and reboot.

### NixOS installs

Set `services.framework-control.enable = false;` (or remove the option) and rebuild:

```bash
sudo nixos-rebuild switch
```

### Manual Uninstall

```bash
sudo systemctl stop framework-control.service
sudo systemctl disable framework-control.service
sudo rm /etc/systemd/system/framework-control.service
sudo rm /usr/local/bin/framework-control
sudo rm -rf /etc/framework-control
sudo systemctl daemon-reload
```

Optionally remove the desktop entry and icon assets:
```bash
rm ~/.local/share/applications/framework-control.desktop
rm -rf ~/.local/share/framework-control
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

### Immutable Distributions (Fedora Silverblue, Bazzite, etc.)

On Fedora Atomic variants, prefer the [dnf repository](#fedora--rhel-dnf) with `rpm-ostree`. The install script uses `/usr/local/bin`, which is typically writable on immutable systems.

## Security Notes

- The service binds to `127.0.0.1` only - it's not accessible from the network
- CORS restricts browser-based cross-origin requests
- The service runs as root because `framework_tool` requires elevated permissions for EC access

## Support

For issues, feature requests, or questions:
- GitHub Issues: https://github.com/ozturkkl/framework-control/issues
- Project README: https://github.com/ozturkkl/framework-control