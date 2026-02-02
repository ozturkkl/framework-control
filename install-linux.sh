#!/usr/bin/env bash
set -Eeuo pipefail

# Framework Control - Linux Installation Script
# Downloads and installs the latest release as a systemd service

REPO="ozturkkl/framework-control"
BINARY_NAME="framework-control"
SERVICE_NAME="framework-control.service"
INSTALL_DIR="/usr/local/bin"
SERVICE_DIR="/etc/systemd/system"
TARBALL_NAME="framework-control-service-x86_64.tar.gz"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

trap 'echo -e "${RED}Installation failed at line $LINENO${NC}" >&2' ERR

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

check_dependencies() {
    local missing=()
    for cmd in curl tar systemctl; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            missing+=("$cmd")
        fi
    done

    if [ ${#missing[@]} -gt 0 ]; then
        error "Missing required commands: ${missing[*]}"
        exit 1
    fi
}

check_framework_tool() {
    if ! command -v framework_tool >/dev/null 2>&1; then
        warn "framework_tool not found on PATH"
        warn "The service will attempt to download it automatically on first run"
        warn "Or install it manually from: https://github.com/FrameworkComputer/framework-system"
        echo "" >&2
    else
        info "framework_tool found: $(command -v framework_tool)"
    fi
}

download_release() {
    local tmpdir
    tmpdir=$(mktemp -d)
    trap 'rm -rf "$tmpdir"' EXIT

    info "Downloading latest release from GitHub..."
    # local download_url="https://github.com/${REPO}/releases/latest/download/${TARBALL_NAME}"
    # FIXME: update this to the real latest release after release
    local download_url="https://github.com/ozturkkl/framework-control/releases/download/0.5.0-beta.2/framework-control-service-x86_64.tar.gz"

    if ! curl -fsSL -o "$tmpdir/$TARBALL_NAME" "$download_url"; then
        error "Failed to download release tarball"
        error "URL: $download_url"
        exit 1
    fi

    info "Extracting files..."
    if ! tar -xzf "$tmpdir/$TARBALL_NAME" -C "$tmpdir"; then
        error "Failed to extract tarball"
        exit 1
    fi

    echo "$tmpdir"
}

install_binary() {
    local tmpdir="$1"

    info "Installing binary to $INSTALL_DIR..."
    if ! install -m 755 "$tmpdir/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"; then
        error "Failed to install binary"
        exit 1
    fi
}

install_service() {
    local tmpdir="$1"

    info "Installing systemd service..."
    if ! install -m 644 "$tmpdir/$SERVICE_NAME" "$SERVICE_DIR/$SERVICE_NAME"; then
        error "Failed to install service file"
        exit 1
    fi

    info "Reloading systemd daemon..."
    systemctl daemon-reload

    info "Enabling service..."
    systemctl enable "$SERVICE_NAME"

    info "Starting service..."
    if ! systemctl start "$SERVICE_NAME"; then
        error "Failed to start service"
        error "Check logs with: sudo journalctl -u $SERVICE_NAME -n 50"
        exit 1
    fi
}

verify_installation() {
    info "Verifying installation..."

    if ! systemctl is-active --quiet "$SERVICE_NAME"; then
        error "Service is not running"
        error "Check logs with: sudo journalctl -u $SERVICE_NAME -n 50"
        exit 1
    fi

    info "Service is running"

    # Give the service a moment to start listening
    sleep 2

    # Extract the actual port from service logs
    local detected_port
    detected_port=$(journalctl -u "$SERVICE_NAME" -n 20 --no-pager 2>/dev/null | grep -oP 'listening on http://127\.0\.0\.1:\K\d+' | tail -1)

    if [ -z "$detected_port" ]; then
        warn "Could not detect port from service logs"
        return
    fi

    # Try to reach the health endpoint on the detected port
    if command -v curl >/dev/null 2>&1; then
        if curl -f -s "http://127.0.0.1:$detected_port/api/health" >/dev/null 2>&1; then
            info "Health check passed"
        else
            warn "Service is running but health check failed"
            warn "The service may still be starting up"
        fi
    fi
}

print_success() {
    # Extract the actual port from service logs
    local port
    port=$(journalctl -u "$SERVICE_NAME" -n 20 --no-pager 2>/dev/null | grep -oP 'listening on http://127\.0\.0\.1:\K\d+' | tail -1)

    local url="http://127.0.0.1:${port:-<configured-port>}"

    echo "" >&2
    echo -e "${GREEN}✓ Installation complete!${NC}" >&2
    echo "" >&2
    echo "Framework Control is now running as a system service." >&2
    echo "" >&2
    echo -e "Access the web UI at: ${GREEN}${url}${NC}" >&2
    echo "" >&2
    echo "Useful commands:" >&2
    echo "  - Check status:  sudo systemctl status $SERVICE_NAME" >&2
    echo "  - View logs:     sudo journalctl -u $SERVICE_NAME -f" >&2
    echo "  - Restart:       sudo systemctl restart $SERVICE_NAME" >&2
    echo "  - Stop:          sudo systemctl stop $SERVICE_NAME" >&2
    echo "" >&2
}

main() {
    info "Framework Control - Linux Installer"
    echo ""

    check_root
    check_dependencies
    check_framework_tool

    local tmpdir
    tmpdir=$(download_release)

    install_binary "$tmpdir"
    install_service "$tmpdir"
    verify_installation
    print_success
}

main "$@"
