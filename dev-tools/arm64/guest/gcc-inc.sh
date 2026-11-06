#!/bin/bash
# In the ARM64 builder: rebuild Settings after a change in cc-man-tweaks.c only.
B=/root/buildenv; S=/mnt/share
mkdir -p $B/mnt/share; mountpoint -q $B/mnt/share || mount --bind $S $B/mnt/share
mountpoint -q $B/proc || mount -t proc proc $B/proc
chroot $B bash -c 'export LC_ALL=C.UTF-8; cp /mnt/share/src/gcc-51/panels/common/cc-man-tweaks.c /w/gcc/panels/common/; ninja -C /w/build > /w/build.log 2>&1 || { tail -20 /w/build.log; exit 1; }; mkdir -p /mnt/share/out; cp /w/build/shell/gnome-control-center /mnt/share/out/gnome-control-center'; echo "GCCINC rc=$?"
