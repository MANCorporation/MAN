#!/bin/bash
# Inside the ARM64 builder: build MAN's patched GNOME Settings binary and the
# PCD (GNOME Web) packages for arm64 in /root/buildenv.
set -euo pipefail
B=/root/buildenv; S=/mnt/share
mkdir -p $B/mnt/share; mountpoint -q $B/mnt/share || mount --bind $S $B/mnt/share
mountpoint -q $B/proc || mount -t proc proc $B/proc
mkdir -p $S/out
cat > $B/root/build.sh <<'IN'
set -euo pipefail
export LC_ALL=C.UTF-8
echo "== Settings"
rm -rf /w/gcc /w/build; mkdir -p /w; cp -a /mnt/share/src/gcc-51 /w/gcc; cd /w/gcc
meson setup /w/build --prefix=/usr --libdir=lib/aarch64-linux-gnu --buildtype=plain \
    -Dlocation-services=enabled -Dprivileged_group=sudo -Ddocumentation=false -Dtests=false -Dmalcontent=true > /w/setup.log 2>&1 || { tail -30 /w/setup.log; exit 1; }
ninja -C /w/build > /w/build.log 2>&1 || { tail -40 /w/build.log; exit 1; }
cp /w/build/shell/gnome-control-center /mnt/share/out/gnome-control-center-arm64
(cd /mnt/share/out && apt-get download gnome-control-center >/dev/null)
echo "== PCD"
rm -rf /w/ep && mkdir -p /w/ep && cd /w/ep
cp /mnt/share/src/epiphany-browser_51.1* .
dpkg-source -x epiphany-browser_51.1-1.dsc src >/dev/null
cd src
cp /mnt/share/src/man-pcd-branding.patch debian/patches/
echo man-pcd-branding.patch >> debian/patches/series
sed -i '/usr\/share\/help/d' debian/epiphany-browser-data.install
{ printf 'epiphany-browser (51.1-1man1) unstable; urgency=medium\n\n  * MAN: rebrand as PCD (name, icon, About, menus, metadata, no help manual).\n\n -- MAN Developers <dev@man.local>  %s\n\n' "$(date -R)"; cat debian/changelog; } > debian/changelog.new
mv debian/changelog.new debian/changelog
DEB_BUILD_OPTIONS="nocheck parallel=16" dpkg-buildpackage -B -uc -us > /w/ep/build.log 2>&1 || { tail -40 /w/ep/build.log; exit 1; }
cp /w/ep/epiphany-browser_*_arm64.deb /mnt/share/out/
echo "APPS DONE"
IN
chroot $B bash /root/build.sh
