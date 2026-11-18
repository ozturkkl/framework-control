#!/bin/sh
set -eu

# Containers and rpm-ostree layering run scriptlets without a live systemd.
[ -d /run/systemd/system ] || exit 0

systemctl daemon-reload

# Full removal deletes the state directory, so it is missing on a first install and on a reinstall.
# The scriptlet arguments can't tell these apart: deb reports a reinstall after apt remove as an upgrade.
if [ ! -d /var/lib/framework-control ]; then
    systemctl enable --now framework-control.service
else
    systemctl try-restart framework-control.service
fi
