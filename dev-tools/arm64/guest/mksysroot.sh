#!/bin/bash
# Inside the ARM64 builder: a sysroot (files only) for cross-compiling MAN's
# Rust/GTK apps on the PC: GTK 4, libadwaita, libc and libgcc development files.
set -euo pipefail
rm -rf /root/sysroot
mmdebstrap --mode=root --arch=arm64 --variant=extract --aptopt='Acquire::ForceIPv4 "true"' \
  --include=libgtk-4-dev,libadwaita-1-dev,libc6-dev,libgcc-16-dev \
  sid /root/sysroot "deb http://deb.debian.org/debian sid main"
tar -C /root/sysroot -cf /mnt/share/sysroot-arm64.tar usr lib 2>/dev/null || tar -C /root/sysroot -cf /mnt/share/sysroot-arm64.tar usr
echo "SYSROOT DONE"
