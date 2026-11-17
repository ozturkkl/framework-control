#!/bin/sh
set -eu

# Containers and rpm-ostree layering run scriptlets without a live systemd.
[ -d /run/systemd/system ] || exit 0

systemctl daemon-reload

# First install: rpm passes 1, deb passes "configure" with no previous version.
if [ "$1" = "1" ] || { [ "$1" = "configure" ] && [ -z "${2:-}" ]; }; then
    systemctl enable --now framework-control.service
else
    systemctl try-restart framework-control.service
fi
