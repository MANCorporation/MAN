#!/bin/sh
# Incremental rebuild after gcc-build-in-vm.sh: sync changed sources, ninja.
set -e
rsync -a --exclude .pc /mnt/src/gcc-51/ /w/gcc/ 2>/dev/null || cp -a /mnt/src/gcc-51/. /w/gcc/
ninja -C /w/build > /w/build.log 2>&1
cp /w/build/shell/gnome-control-center /mnt/src/out/gnome-control-center
