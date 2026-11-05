#!/bin/bash
# In a VM of the image, with disk vdb: sid chroot with the build dependencies of Settings; builds gcc-51.
set -euo pipefail
B=/mnt/b
mountpoint -q $B || { mkdir -p $B; mkfs.ext4 -q -F /dev/vdb; mount /dev/vdb $B; }
if [ ! -e $B/env/usr/bin/meson ]; then
  echo 'Acquire::ForceIPv4 "true";' > /etc/apt/apt.conf.d/99ipv4-build
  apt-get update -q >/dev/null
  DEBIAN_FRONTEND=noninteractive apt-get install -y -q --no-install-recommends mmdebstrap >/dev/null 2>&1
  mmdebstrap --mode=root --variant=buildd --aptopt='Acquire::ForceIPv4 "true"' --include=ca-certificates,dpkg-dev,meson,ninja-build sid $B/env "deb http://deb.debian.org/debian sid main" "deb-src http://deb.debian.org/debian sid main"
  chroot $B/env apt-get -o Acquire::ForceIPv4=true update -q >/dev/null
  chroot $B/env env DEBIAN_FRONTEND=noninteractive apt-get -o Acquire::ForceIPv4=true build-dep -y -q gnome-control-center >/dev/null
  rm -f /etc/apt/apt.conf.d/99ipv4-build
fi
mkdir -p $B/env/s; mountpoint -q $B/env/s || mount --bind /mnt/w $B/env/s
mountpoint -q $B/env/proc || mount -t proc proc $B/env/proc
chroot $B/env bash -c 'export LC_ALL=C.UTF-8; rm -rf /w; mkdir -p /w; cp -a /s/src/gcc-51 /w/gcc; cd /w/gcc; meson setup /w/build --prefix=/usr --libdir=lib/x86_64-linux-gnu --buildtype=plain -Dlocation-services=enabled -Dprivileged_group=sudo -Ddocumentation=false -Dtests=false -Dmalcontent=true > /w/setup.log 2>&1 || { tail -30 /w/setup.log; exit 1; }; ninja -C /w/build > /w/build.log 2>&1 || { grep -E "error|FAILED" -A4 /w/build.log | head -40; exit 1; }; cp /w/build/shell/gnome-control-center /s/gnome-control-center'
echo GCC DONE
