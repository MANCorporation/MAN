#!/bin/bash
# Inside the ARM64 builder: a sid arm64 build chroot with the build
# dependencies of GNOME Settings and GNOME Web (MAN patches them).
set -euo pipefail
B=/root/buildenv
rm -rf $B
mmdebstrap --mode=root --arch=arm64 --variant=buildd --aptopt='Acquire::ForceIPv4 "true"' \
  --include=ca-certificates,dpkg-dev,devscripts,meson,ninja-build \
  --setup-hook='mkdir -p "$1/etc/apt/sources.list.d"' \
  sid $B "deb http://deb.debian.org/debian sid main" "deb-src http://deb.debian.org/debian sid main"
chroot $B apt-get -o Acquire::ForceIPv4=true update -q
chroot $B env DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::ForceIPv4=true build-dep -y -q gnome-control-center epiphany-browser
echo "BUILDENV DONE"
