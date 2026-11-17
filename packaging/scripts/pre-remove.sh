#!/bin/sh
set -eu

[ -d /run/systemd/system ] || exit 0

# Full removal: rpm passes 0, deb passes "remove". Upgrades keep the service running.
if [ "$1" = "0" ] || [ "$1" = "remove" ]; then
    systemctl disable --now framework-control.service
fi
