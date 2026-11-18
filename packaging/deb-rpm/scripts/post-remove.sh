#!/bin/sh
set -eu

# Full removal: rpm passes 0, deb passes "remove" and later "purge".
case "$1" in
    0 | remove)
        rm -rf /var/lib/framework-control
        ;;
    purge)
        rm -rf /var/lib/framework-control /etc/framework-control
        ;;
    *)
        exit 0
        ;;
esac

[ -d /run/systemd/system ] || exit 0
systemctl daemon-reload
