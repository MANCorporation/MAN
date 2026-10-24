#!/bin/sh
# Run as root inside the build VM (src shared at /mnt/src): builds the patched
# Settings binary into /mnt/src/out/gnome-control-center.
set -e
rm -rf /w/gcc /w/build
cp -a /mnt/src/gcc-51 /w/gcc
cd /w/gcc
meson setup /w/build --prefix=/usr --libdir=lib/x86_64-linux-gnu --buildtype=plain \
    -Dlocation-services=enabled -Dprivileged_group=sudo -Ddocumentation=false -Dtests=false -Dmalcontent=true > /w/setup.log 2>&1
ninja -C /w/build > /w/build.log 2>&1
cp /w/build/shell/gnome-control-center /mnt/src/out/gnome-control-center
