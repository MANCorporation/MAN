#!/bin/bash
# In the ARM64 builder: MAN's Settings binary for arm64.
set -euo pipefail
B=/root/buildenv; S=/mnt/share
mkdir -p $B/mnt/share; mountpoint -q $B/mnt/share || mount --bind $S $B/mnt/share
mountpoint -q $B/proc || mount -t proc proc $B/proc
mkdir -p $S/out
chroot $B bash -c 'set -e; export LC_ALL=C.UTF-8; rm -rf /w/gcc /w/build; mkdir -p /w; cp -a /mnt/share/src/gcc-51 /w/gcc; cd /w/gcc; meson setup /w/build --prefix=/usr --libdir=lib/aarch64-linux-gnu --buildtype=plain -Dlocation-services=enabled -Dprivileged_group=sudo -Ddocumentation=false -Dtests=false -Dmalcontent=true > /w/setup.log 2>&1 || { tail -30 /w/setup.log; exit 1; }; ninja -C /w/build > /w/build.log 2>&1 || { tail -40 /w/build.log; exit 1; }; cp /w/build/shell/gnome-control-center /mnt/share/out/gnome-control-center-arm64'
echo GCCARM DONE
