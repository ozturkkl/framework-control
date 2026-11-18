#!/usr/bin/env bash
# Usage: write-repo-file.sh <deb|rpm> <stable|beta> <base-url>
# Prints the apt .sources or dnf .repo file for a channel. The packages ship the stable ones,
# which must match what Pages serves byte for byte so rpm does not leave .rpmnew files.
set -Eeuo pipefail
trap 'echo "write-repo-file.sh failed at line $LINENO: $BASH_COMMAND" >&2' ERR

format=$1
channel=$2
base_url=$3

case "$format" in
    deb)
        cat <<EOF
Types: deb
URIs: $base_url/deb/$channel/
Suites: ./
Signed-By: /usr/share/keyrings/framework-control-archive-keyring.gpg
EOF
        ;;
    rpm)
        cat <<EOF
[framework-control-$channel]
name=Framework Control ($channel)
baseurl=$base_url/rpm/$channel/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=$base_url/framework-control.asc
EOF
        ;;
esac
