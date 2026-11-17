#!/usr/bin/env bash
# Usage: build-packages.sh <output-dir>
# Packages the output of web/scripts/build-linux.mjs as framework-control-service-x86_64.{deb,rpm}.
set -Eeuo pipefail
trap 'echo "build-packages.sh failed at line $LINENO: $BASH_COMMAND" >&2' ERR

output_dir=$(realpath -m "$1")
repo_root=$(realpath "$(dirname "$0")/..")

: "${PACKAGE_VERSION:?}" "${PACKAGE_RELEASE:?}" "${PACKAGE_SIGNING_KEY_FILE:?}" "${FRAMEWORK_CONTROL_PORT:?}"

cd "$repo_root"
sed "s/{PORT}/$FRAMEWORK_CONTROL_PORT/" packaging/framework-control.desktop.in \
    > service/target/dist-linux/framework-control.desktop

mkdir -p "$output_dir"
for packager in deb rpm; do
    nfpm package --config packaging/nfpm.yaml --packager "$packager" \
        --target "$output_dir/framework-control-service-x86_64.$packager"
done
