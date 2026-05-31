#!/usr/bin/env bash
set -Eeuo pipefail

# Framework Control - Linux Uninstallation Script
# Removes the systemd service, binary, config, and desktop integration

BINARY_NAME="framework-control"
SERVICE_NAME="framework-control.service"
INSTALL_DIR="/usr/local/bin"
SERVICE_DIR="/etc/systemd/system"
CONFIG_DIR="/etc/framework-control"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

trap 'echo -e "${RED}Uninstallation failed at line $LINENO${NC}" >&2' ERR

info() {
    echo -e "${GREEN}[INFO]${NC} $*" >&2
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $*" >&2
}

error() {
    echo -e "${RED}[ERROR]${NC} $*" >&2
}

check_root() {
    if [ "$EUID" -ne 0 ]; then
        error "This script must be run as root (use sudo)"
        exit 1
    fi
}

# Mirror the service's user-home detection (see service/src/utils/fs.rs) so we can
# remove the per-user desktop entry and icon even when invoked via sudo.
detect_user_home() {
    if [ -n "${SUDO_USER:-}" ]; then
        echo "/home/$SUDO_USER"
        return
    fi

    if [ -d /run/user ]; then
        for dir in /run/user/*; do
            local uid
            uid=$(basename "$dir")
            case "$uid" in
                '' | *[!0-9]*) continue ;;
            esac
            if [ "$uid" -ge 1000 ]; then
                local username
                if username=$(id -un "$uid" 2>/dev/null) && [ -n "$username" ]; then
                    echo "/home/$username"
                    return
                fi
            fi
        done
    fi

    echo "${HOME:-}"
}

remove_service() {
    if [ ! -f "$SERVICE_DIR/$SERVICE_NAME" ] && ! systemctl list-unit-files "$SERVICE_NAME" >/dev/null 2>&1; then
        info "Service not installed, skipping"
        return
    fi

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        info "Stopping service..."
        systemctl stop "$SERVICE_NAME"
    fi

    if systemctl is-enabled --quiet "$SERVICE_NAME"; then
        info "Disabling service..."
        systemctl disable "$SERVICE_NAME"
    fi

    if [ -f "$SERVICE_DIR/$SERVICE_NAME" ]; then
        info "Removing service file..."
        rm -f "$SERVICE_DIR/$SERVICE_NAME"
    fi

    info "Reloading systemd daemon..."
    systemctl daemon-reload
}

remove_binary() {
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        info "Removing binary..."
        rm -f "$INSTALL_DIR/$BINARY_NAME"
    fi

    # Shortcut-creation marker dropped by the installer
    rm -f "$INSTALL_DIR/create_shortcuts.flag"
}

remove_config() {
    if [ -d "$CONFIG_DIR" ]; then
        info "Removing config directory $CONFIG_DIR..."
        rm -rf "$CONFIG_DIR"
    fi
}

remove_desktop_integration() {
    local user_home
    user_home=$(detect_user_home)

    if [ -z "$user_home" ]; then
        warn "Could not determine user home; skipping desktop entry/icon cleanup"
        return
    fi

    local desktop_file="$user_home/.local/share/applications/framework-control.desktop"
    local asset_dir="$user_home/.local/share/framework-control"

    if [ -f "$desktop_file" ]; then
        info "Removing desktop entry..."
        rm -f "$desktop_file"
    fi

    if [ -d "$asset_dir" ]; then
        info "Removing icon assets..."
        rm -rf "$asset_dir"
    fi
}

print_success() {
    echo "" >&2
    echo -e "${GREEN}✓ Uninstallation complete!${NC}" >&2
    echo "" >&2
    echo "Framework Control has been removed from this system." >&2
    echo "" >&2
    echo "Note: framework_tool (the Framework CLI) was installed separately and was left in place." >&2
    echo "" >&2
}

main() {
    info "Framework Control - Linux Uninstaller"
    echo ""

    check_root
    remove_service
    remove_binary
    remove_config
    remove_desktop_integration
    print_success
}

main "$@"
