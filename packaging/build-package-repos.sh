#!/usr/bin/env bash
# Usage: build-package-repos.sh <site-dir> <base-url>
# Builds signed apt and dnf repos from the packages attached to recent GitHub releases.
# stable: the newest stable releases. beta: the newest releases of any kind.
# The signing key must already be in the default gpg keyring.
set -Eeuo pipefail
trap 'echo "build-package-repos.sh failed at line $LINENO: $BASH_COMMAND" >&2' ERR

site_dir=$(realpath -m "$1")
base_url=$2
releases_per_channel=3
package_asset=framework-control-service-x86_64

releases_with_packages=$(gh api "repos/$GITHUB_REPOSITORY/releases?per_page=50" --jq \
    "[.[] | select(.draft | not) | select(any(.assets[]; .name == \"$package_asset.deb\"))]")

release_tags() {
    local channel=$1 filter
    case "$channel" in
        stable) filter='map(select(.prerelease | not))' ;;
        beta) filter='.' ;;
    esac
    jq -r "$filter | .[:$releases_per_channel] | .[].tag_name" <<< "$releases_with_packages"
}

download_packages() {
    local channel=$1 tag
    mkdir -p "$site_dir/deb/$channel" "$site_dir/rpm/$channel"
    for tag in $(release_tags "$channel"); do
        gh release download "$tag" --repo "$GITHUB_REPOSITORY" --pattern "$package_asset.deb" \
            --output "$site_dir/deb/$channel/framework-control-$tag.deb"
        gh release download "$tag" --repo "$GITHUB_REPOSITORY" --pattern "$package_asset.rpm" \
            --output "$site_dir/rpm/$channel/framework-control-$tag.rpm"
    done
}

write_apt_repo() {
    local channel=$1
    cd "$site_dir/deb/$channel"
    apt-ftparchive packages . > Packages
    gzip -9 --keep Packages
    # Written elsewhere first because apt-ftparchive would hash a partial Release file.
    local release_file
    release_file=$(mktemp)
    apt-ftparchive \
        -o "APT::FTPArchive::Release::Origin=framework-control" \
        -o "APT::FTPArchive::Release::Label=Framework Control" \
        -o "APT::FTPArchive::Release::Suite=$channel" \
        release . > "$release_file"
    install -m 644 "$release_file" Release
    rm "$release_file"
    gpg --batch --yes --clearsign --output InRelease Release
    gpg --batch --yes --armor --detach-sign --output Release.gpg Release

    cat > framework-control.sources <<EOF
Types: deb
URIs: $base_url/deb/$channel/
Suites: ./
Signed-By: /etc/apt/keyrings/framework-control.asc
EOF
}

write_dnf_repo() {
    local channel=$1
    cd "$site_dir/rpm/$channel"
    createrepo_c --quiet .
    gpg --batch --yes --armor --detach-sign --output repodata/repomd.xml.asc repodata/repomd.xml

    cat > framework-control.repo <<EOF
[framework-control-$channel]
name=Framework Control ($channel)
baseurl=$base_url/rpm/$channel/
enabled=1
gpgcheck=1
repo_gpgcheck=1
gpgkey=$base_url/framework-control.asc
EOF
}

mkdir -p "$site_dir"
gpg --batch --yes --armor --export --output "$site_dir/framework-control.asc"

for channel in stable beta; do
    download_packages "$channel"
    write_apt_repo "$channel"
    write_dnf_repo "$channel"
done
