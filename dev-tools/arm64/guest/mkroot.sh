#!/bin/bash
# Inside the ARM64 builder: Debian sid arm64 with the MAN package set onto /dev/vdb.
set -euo pipefail
T=/mnt/target; DEV=/dev/vdb
umount -R $T 2>/dev/null || true
mkfs.ext4 -q -F -L MAN $DEV
mkdir -p $T; mount $DEV $T
mmdebstrap --mode=root --arch=arm64 --variant=apt \
  --components=main,contrib,non-free,non-free-firmware \
  --aptopt='Acquire::ForceIPv4 "true"' --aptopt='APT::Install-Recommends "false"' \
  --dpkgopt=force-unsafe-io \
  --include="$(paste -sd, /mnt/share/pkgs-arm64.txt)" \
  sid $T "deb http://deb.debian.org/debian sid main contrib non-free non-free-firmware"
echo "MKROOT DONE"; df -h $T | tail -1
