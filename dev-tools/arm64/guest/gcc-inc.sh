#!/bin/bash
# In the ARM64 builder: rebuild Settings from /mnt/share/src/gcc-51 in the existing build
# tree (/w/build of gcc-arm.sh); meson reconfigures by itself when meson.build files changed.
B=/root/buildenv; S=/mnt/share
mkdir -p $B/mnt/share; mountpoint -q $B/mnt/share || mount --bind $S $B/mnt/share
mountpoint -q $B/proc || mount -t proc proc $B/proc
chroot $B bash -c 'export LC_ALL=C.UTF-8; cp -a /mnt/share/src/gcc-51/. /w/gcc/; ninja -C /w/build > /w/build.log 2>&1 || { tail -20 /w/build.log; exit 1; }; mkdir -p /mnt/share/out; cp /w/build/shell/gnome-control-center /mnt/share/out/gnome-control-center'; echo "GCCINC rc=$?"
