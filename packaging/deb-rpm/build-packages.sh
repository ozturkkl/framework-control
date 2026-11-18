#!/usr/bin/env bash
# Usage: build-packages.sh <output-dir>
# Packages the output of web/scripts/build-linux.mjs as framework-control-service-x86_64.{deb,rpm}.
# PACKAGE_REPO_URL must match the base URL build-package-repos.sh publishes to.
set -Eeuo pipefail
trap 'echo "build-packages.sh failed at line $LINENO: $BASH_COMMAND" >&2' ERR

output_dir=$(realpath -m "$1")
repo_root=$(realpath "$(dirname "$0")/../..")

: "${PACKAGE_VERSION:?}" "${PACKAGE_RELEASE:?}" "${PACKAGE_SIGNING_KEY_FILE:?}" "${PACKAGE_REPO_URL:?}" "${FRAMEWORK_CONTROL_PORT:?}"

cd "$repo_root"
staging_dir=service/target/dist-linux

sed "s/{PORT}/$FRAMEWORK_CONTROL_PORT/" packaging/deb-rpm/framework-control.desktop.in \
    > "$staging_dir/framework-control.desktop"

gnupg_home=$(mktemp -d)
gpg --homedir "$gnupg_home" --batch --quiet --import "$PACKAGE_SIGNING_KEY_FILE"
gpg --homedir "$gnupg_home" --batch --yes --export --output "$staging_dir/framework-control-archive-keyring.pgp"
rm -r "$gnupg_home"

packaging/deb-rpm/write-repo-file.sh deb stable "$PACKAGE_REPO_URL" > "$staging_dir/framework-control.sources"
packaging/deb-rpm/write-repo-file.sh rpm stable "$PACKAGE_REPO_URL" > "$staging_dir/framework-control.repo"

mkdir -p "$output_dir"
for packager in deb rpm; do
    nfpm package --config packaging/deb-rpm/nfpm.yaml --packager "$packager" \
        --target "$output_dir/framework-control-service-x86_64.$packager"
done
